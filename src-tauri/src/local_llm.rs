//! Local LLM provider via Ollama's HTTP API.
//!
//! Mirrors the four functions the Gemini and Claude-CLI providers expose so
//! the pipeline can dispatch on `ai_provider == "local"` without caring
//! which one is in use.
//!
//! Uses streaming (`stream: true`) so we can emit token-rate progress to
//! the frontend during the long-running calls — a Q4 8B on a CPU laptop
//! takes minutes, and silent waits are awful UX.
//!
//! Thinking mode is **disabled by default** (`think: false` request option).
//! qwen3 etc. otherwise spend many seconds emitting `<think>...</think>`
//! reasoning that the curation/conflict tasks don't benefit from.

use crate::config::LocalLlmConfig;
use crate::gemini::{
    parse_conflict_flags_json, parse_conflict_queries_json, parse_keyword_json, ConflictFlag,
    ConflictQueries, KeywordResult, Paper,
};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

const DEFAULT_HOST: &str = "http://127.0.0.1:11434";
const PROGRESS_EVENT: &str = "newsletter-progress";

/// Per-request override knobs we actually expose to the user.
#[derive(Debug, Clone)]
pub struct CallOptions<'a> {
    pub temperature: f32,
    pub num_predict: i32,
    /// Topic / profile name used for progress event routing.
    pub topic: &'a str,
    /// Step label for progress events (e.g. "summarize", "scan").
    pub step: &'a str,
}

/// Streaming response chunk from Ollama `/api/generate`.
#[derive(Debug, Deserialize)]
struct GenerateChunk {
    #[serde(default)]
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    eval_duration: Option<u64>, // nanoseconds
    #[serde(default)]
    error: Option<String>,
}

/// Plain non-streaming call result we return to callers (full text + a
/// summary of perf metadata).
#[derive(Debug, Clone, Serialize)]
pub struct CallResult {
    pub text: String,
    pub eval_count: u64,
    pub elapsed_ms: u64,
    pub tokens_per_sec: f64,
}

/// Test that the Ollama daemon is reachable and return its version string.
pub async fn check_status(client: &Client, host: &str) -> Result<String, String> {
    let url = format!("{}/api/version", host_or_default(host));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Ollama not reachable at {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama version endpoint returned {}", resp.status()));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama version: {e}"))?;
    Ok(v["version"].as_str().unwrap_or("unknown").to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelInfo {
    pub name: String,
    pub size_bytes: u64,
}

pub async fn list_models(client: &Client, host: &str) -> Result<Vec<LocalModelInfo>, String> {
    let url = format!("{}/api/tags", host_or_default(host));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Ollama list models failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama tags endpoint returned {}", resp.status()));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama models: {e}"))?;
    let arr = v["models"].as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .filter_map(|m| {
            Some(LocalModelInfo {
                name: m["name"].as_str()?.to_string(),
                size_bytes: m["size"].as_u64().unwrap_or(0),
            })
        })
        .collect())
}

/// Pull a model via `/api/pull`. Streams progress lines and emits them
/// to the frontend on the `local-pull-progress` channel.
///
/// Respects an optional cancel signal so the UI can stop a runaway pull.
/// On cancel, emits one final progress event with `status: "cancelled"`
/// before returning `Err("cancelled")`.
pub async fn pull_model(
    app: &AppHandle,
    client: &Client,
    host: &str,
    model: &str,
    mut cancel_rx: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<(), String> {
    let url = format!("{}/api/pull", host_or_default(host));
    let resp = client
        .post(&url)
        .json(&json!({ "model": model, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("Ollama pull failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama pull returned {}", resp.status()));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = Vec::<u8>::new();

    loop {
        let next_chunk: Option<Result<bytes::Bytes, reqwest::Error>> =
            match cancel_rx.as_mut() {
                Some(rx) => tokio::select! {
                    next = stream.next() => next,
                    _ = rx => {
                        let _ = app.emit(
                            "local-pull-progress",
                            json!({
                                "model": model,
                                "status": "cancelled",
                                "total": Value::Null,
                                "completed": Value::Null,
                            }),
                        );
                        return Err("cancelled".to_string());
                    }
                },
                None => stream.next().await,
            };
        let chunk = match next_chunk {
            Some(c) => c,
            None => break,
        };
        let bytes = chunk.map_err(|e| format!("pull stream read: {e}"))?;
        buf.extend_from_slice(&bytes);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line = buf.drain(..=pos).collect::<Vec<_>>();
            let text = String::from_utf8_lossy(&line).trim().to_string();
            if text.is_empty() {
                continue;
            }
            let v: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let status = v["status"].as_str().unwrap_or("").to_string();
            let total = v["total"].as_u64();
            let completed = v["completed"].as_u64();
            let _ = app.emit(
                "local-pull-progress",
                json!({
                    "model": model,
                    "status": status,
                    "total": total,
                    "completed": completed,
                }),
            );
            if let Some(err) = v["error"].as_str() {
                return Err(format!("pull error: {err}"));
            }
        }
    }
    Ok(())
}

fn host_or_default(host: &str) -> String {
    if host.trim().is_empty() {
        DEFAULT_HOST.to_string()
    } else {
        host.trim_end_matches('/').to_string()
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Streaming generate — the workhorse
// ════════════════════════════════════════════════════════════════════════════

async fn streaming_generate(
    app: Option<&AppHandle>,
    client: &Client,
    cfg: &LocalLlmConfig,
    prompt: &str,
    opts: &CallOptions<'_>,
) -> Result<CallResult, String> {
    let url = format!("{}/api/generate", host_or_default(&cfg.host));
    let body = json!({
        "model": cfg.model,
        "prompt": prompt,
        "stream": true,
        "think": false,
        "options": {
            "temperature": opts.temperature,
            "num_predict": opts.num_predict,
            "num_ctx": cfg.num_ctx,
        }
    });

    let started = Instant::now();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama API error {status}: {text}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = Vec::<u8>::new();
    let mut text = String::new();
    let mut eval_count: u64 = 0;
    let mut eval_duration_ns: u64 = 0;
    let mut last_emit = Instant::now();
    let mut tokens_seen: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("Ollama stream read: {e}"))?;
        buf.extend_from_slice(&bytes);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line_str = String::from_utf8_lossy(&line);
            let trimmed = line_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            let chunk_obj: GenerateChunk = match serde_json::from_str(trimmed) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if let Some(err) = chunk_obj.error {
                return Err(format!("Ollama stream error: {err}"));
            }
            text.push_str(&chunk_obj.response);
            tokens_seen += 1;

            if chunk_obj.done {
                eval_count = chunk_obj.eval_count.unwrap_or(0);
                eval_duration_ns = chunk_obj.eval_duration.unwrap_or(0);
            }

            // Throttle progress emits to ~3 per second.
            if let Some(handle) = app {
                if last_emit.elapsed().as_millis() >= 333 || chunk_obj.done {
                    let elapsed = started.elapsed();
                    let tps = if elapsed.as_secs_f64() > 0.0 {
                        tokens_seen as f64 / elapsed.as_secs_f64()
                    } else {
                        0.0
                    };
                    let msg = format!(
                        "Local model: {} tok @ {:.1} tok/s",
                        tokens_seen, tps
                    );
                    let _ = handle.emit(
                        PROGRESS_EVENT,
                        json!({
                            "topic": opts.topic,
                            "step": opts.step,
                            "message": msg,
                            "done": false,
                            "error": false,
                            "tokens_generated": tokens_seen,
                            "tokens_per_sec": tps,
                            "elapsed_ms": elapsed.as_millis() as u64,
                        }),
                    );
                    last_emit = Instant::now();
                }
            }
        }
    }

    let elapsed_ms = started.elapsed().as_millis() as u64;
    let tokens_per_sec = if eval_duration_ns > 0 {
        eval_count as f64 / (eval_duration_ns as f64 / 1e9)
    } else if elapsed_ms > 0 {
        (tokens_seen as f64 * 1000.0) / elapsed_ms as f64
    } else {
        0.0
    };

    Ok(CallResult {
        text,
        eval_count: if eval_count > 0 { eval_count } else { tokens_seen },
        elapsed_ms,
        tokens_per_sec,
    })
}

// ════════════════════════════════════════════════════════════════════════════
//  Public API — mirrors gemini.rs
// ════════════════════════════════════════════════════════════════════════════

pub async fn extract_keywords(
    client: &Client,
    cfg: &LocalLlmConfig,
    query: &str,
) -> Result<KeywordResult, String> {
    let prompt = build_keyword_prompt(query);
    let res = streaming_generate(
        None,
        client,
        cfg,
        &prompt,
        &CallOptions {
            temperature: 0.2,
            num_predict: 256,
            topic: query,
            step: "keywords",
        },
    )
    .await?;
    parse_keyword_json(&res.text, query)
}

pub async fn summarize(
    app: Option<&AppHandle>,
    client: &Client,
    cfg: &LocalLlmConfig,
    papers: &[Paper],
    user_query: &str,
) -> Result<String, String> {
    let prompt = build_curation_prompt(papers, user_query);
    let res = streaming_generate(
        app,
        client,
        cfg,
        &prompt,
        &CallOptions {
            temperature: cfg.temperature_curation,
            num_predict: cfg.num_predict_curation,
            topic: user_query,
            step: "summarize",
        },
    )
    .await?;
    Ok(strip_think_and_fences(&res.text))
}

pub async fn extract_conflict_queries(
    client: &Client,
    cfg: &LocalLlmConfig,
    research_description: &str,
    key_terms: &[String],
) -> Result<ConflictQueries, String> {
    let prompt = build_conflict_query_prompt(research_description, key_terms);
    let res = streaming_generate(
        None,
        client,
        cfg,
        &prompt,
        &CallOptions {
            temperature: 0.2,
            num_predict: 512,
            topic: research_description,
            step: "conflict-queries",
        },
    )
    .await?;
    parse_conflict_queries_json(&res.text, research_description, key_terms)
}

pub async fn evaluate_conflicts(
    app: Option<&AppHandle>,
    client: &Client,
    cfg: &LocalLlmConfig,
    papers: &[Paper],
    research_description: &str,
    key_terms: &[String],
) -> Result<Vec<ConflictFlag>, String> {
    if papers.is_empty() {
        return Ok(Vec::new());
    }
    let prompt = build_conflict_eval_prompt(papers, research_description, key_terms);
    let res = streaming_generate(
        app,
        client,
        cfg,
        &prompt,
        &CallOptions {
            temperature: cfg.temperature_scan,
            num_predict: cfg.num_predict_scan,
            topic: research_description,
            step: "scan",
        },
    )
    .await?;
    parse_conflict_flags_json(&res.text)
}

// ════════════════════════════════════════════════════════════════════════════
//  Prompt builders — copied from gemini.rs almost verbatim so the
//  baseline comparison stays apples-to-apples.
// ════════════════════════════════════════════════════════════════════════════

fn build_keyword_prompt(query: &str) -> String {
    format!(
        r#"Convert this research interest into optimal academic search keywords.

User interest: "{query}"

Return ONLY a JSON object with two keys:
  "keywords": a short search string (3-8 words, no special chars)
  "title": a concise newsletter title (3-5 words)

Example: {{"keywords": "large language models drug discovery", "title": "LLMs in Drug Discovery"}}

Respond with valid JSON only, no markdown fences."#
    )
}

fn build_curation_prompt(papers: &[Paper], user_query: &str) -> String {
    let paper_list: String = papers
        .iter()
        .take(60)
        .enumerate()
        .map(|(i, p)| {
            let abstract_preview = if p.abstract_text.len() > 600 {
                &p.abstract_text[..600]
            } else {
                &p.abstract_text
            };
            format!(
                "Paper {}: {} by {}.\nAbstract: {}\nURL: {}  Source: {}",
                i + 1,
                p.title,
                p.authors,
                abstract_preview,
                p.url,
                p.source
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    let n = papers.len().min(60);
    format!(
        r#"You are a research newsletter curator. The reader is interested in:
"{user_query}"

Below are {n} papers from academic databases.

Select the 10 most relevant and impactful papers. For each write:
- A clear, jargon-free 2-3 sentence summary (what it does, why it matters)
- A relevance tag (e.g. Machine Learning, Biology, Climate, Economics…)

End with a 2-sentence overview of the theme across these papers.

Format as Markdown: ## for titles linked to paper URL, paragraphs for summaries, **bold** for tags. Do NOT wrap in code fences.

{paper_list}"#
    )
}

fn build_conflict_query_prompt(research_description: &str, key_terms: &[String]) -> String {
    let terms_str = key_terms.join(", ");
    format!(
        r#"You are an academic research analyst. Given a researcher's description of their work and key terms, generate multiple search query strings to find potentially competing or overlapping papers.

Research description: "{research_description}"

Key terms: [{terms_str}]

Generate the following:
1. A "primary" query: the most specific search string targeting the exact research topic (3-8 words)
2. "method_queries": 2-3 search strings focusing on the specific methods, techniques, or approaches mentioned (3-8 words each)
3. A "broad_query": a broader domain-level search string that captures related work in the general field (3-6 words)

Return ONLY a JSON object with these keys:
  "primary": "...",
  "method_queries": ["...", "...", "..."],
  "broad_query": "..."

Respond with valid JSON only, no markdown fences."#
    )
}

fn build_conflict_eval_prompt(
    papers: &[Paper],
    research_description: &str,
    key_terms: &[String],
) -> String {
    let terms_str = key_terms.join(", ");
    let paper_list: String = papers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let abstract_preview = if p.abstract_text.len() > 1200 {
                &p.abstract_text[..1200]
            } else {
                &p.abstract_text
            };
            format!(
                "[{}] Title: {}\nAuthors: {}\nDate: {}\nURL: {}\nAbstract: {}",
                i + 1,
                p.title,
                p.authors,
                p.date,
                p.url,
                abstract_preview
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    let n = papers.len();
    format!(
        r#"You are an expert academic conflict/overlap detector. A researcher needs to know which recent papers overlap with their specific work.

RESEARCHER'S WORK:
"{research_description}"

KEY TERMS: [{terms_str}]

Below are {n} papers to evaluate. Score each paper 0-100 on how much it overlaps with or competes with the researcher's work:
- 0-24: No meaningful overlap
- 25-49: Some topical overlap but different focus (low threat)
- 50-74: Significant methodological or topical overlap (moderate threat)
- 75-89: High overlap, potentially competing work (significant threat)
- 90-100: Very high overlap, nearly identical research direction (critical threat)

IMPORTANT: Err on the side of INCLUDING papers rather than excluding. A false positive is far less costly than a false negative. If in doubt, score higher.

For EVERY paper scoring 25 or above, return a JSON object. Return a JSON array of these objects.

Each object must have:
- "paper_index": the paper number (1-based integer)
- "score": overlap score (integer 0-100)
- "title": the paper's title
- "url": the paper's URL
- "authors": the paper's authors
- "date": the paper's date
- "summary": 1-2 sentence summary of the paper
- "overlap": explanation of how it overlaps with the researcher's work
- "difference": what makes it different from the researcher's work
- "threat_level": one of "low", "moderate", "significant", "critical"
- "matched_terms": array of key terms from the researcher's list that match

If no papers score 25 or above, return an empty JSON array: []

Respond with ONLY the JSON array, no markdown fences.

PAPERS:
{paper_list}"#
    )
}

/// Some local models still emit a stray `<think>...</think>` block even with
/// `think: false` (older Ollama versions). Strip it, then strip code fences,
/// the same way we do for Gemini output.
fn strip_think_and_fences(text: &str) -> String {
    let mut out = text.to_string();
    if let Some(start) = out.find("<think>") {
        if let Some(end) = out.find("</think>") {
            if end > start {
                out = format!("{}{}", &out[..start], &out[end + "</think>".len()..]);
            }
        }
    }
    let trimmed = out.trim();
    let mut s = trimmed.to_string();
    if s.starts_with("```") {
        if let Some(nl) = s.find('\n') {
            s = s[nl + 1..].to_string();
        }
    }
    if s.ends_with("```") {
        s = s[..s.len() - 3].trim_end().to_string();
    }
    s
}

// ════════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_default_handles_empty() {
        assert_eq!(host_or_default(""), DEFAULT_HOST);
        assert_eq!(host_or_default("  "), DEFAULT_HOST);
    }

    #[test]
    fn host_default_strips_trailing_slash() {
        assert_eq!(host_or_default("http://localhost:11434/"), "http://localhost:11434");
    }

    #[test]
    fn keyword_prompt_contains_user_query() {
        let p = build_keyword_prompt("machine learning genomics");
        assert!(p.contains("machine learning genomics"));
        assert!(p.contains("JSON"));
        assert!(p.contains("keywords"));
        assert!(p.contains("title"));
    }

    #[test]
    fn curation_prompt_includes_papers_and_query() {
        let papers = vec![
            Paper {
                title: "Test paper".to_string(),
                abstract_text: "An abstract".to_string(),
                authors: "Doe".to_string(),
                date: "2025-01-01".to_string(),
                url: "https://x".to_string(),
                source: "OpenAlex".to_string(),
            },
        ];
        let p = build_curation_prompt(&papers, "deep learning");
        assert!(p.contains("deep learning"));
        assert!(p.contains("Test paper"));
        assert!(p.contains("Select the 10 most relevant"));
    }

    #[test]
    fn curation_prompt_truncates_to_60_papers() {
        let papers: Vec<Paper> = (0..80)
            .map(|i| Paper {
                title: format!("Paper {i}"),
                abstract_text: "abc".to_string(),
                authors: "x".to_string(),
                date: String::new(),
                url: String::new(),
                source: String::new(),
            })
            .collect();
        let p = build_curation_prompt(&papers, "topic");
        assert!(p.contains("Paper 60"));
        assert!(!p.contains("Paper 61:"));
    }

    #[test]
    fn conflict_eval_prompt_lists_terms() {
        let papers = vec![Paper {
            title: "T".to_string(),
            abstract_text: "A".to_string(),
            authors: "x".to_string(),
            date: String::new(),
            url: String::new(),
            source: String::new(),
        }];
        let p = build_conflict_eval_prompt(
            &papers,
            "I work on FH SAE",
            &["fay-herriot".to_string(), "INLA".to_string()],
        );
        assert!(p.contains("fay-herriot"));
        assert!(p.contains("INLA"));
        assert!(p.contains("0-100"));
        assert!(p.contains("threat_level"));
    }

    #[test]
    fn strip_removes_think_block() {
        let text = "<think>let me reason</think>\n## Final\nbody";
        let out = strip_think_and_fences(text);
        assert!(!out.contains("<think>"));
        assert!(out.contains("Final"));
    }

    #[test]
    fn strip_removes_code_fences() {
        let text = "```markdown\nbody here\n```";
        let out = strip_think_and_fences(text);
        assert_eq!(out.trim(), "body here");
    }

    #[test]
    fn strip_handles_clean_text() {
        let text = "## Heading\nplain markdown";
        let out = strip_think_and_fences(text);
        assert_eq!(out, text);
    }
}
