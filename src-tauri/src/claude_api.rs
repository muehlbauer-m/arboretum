//! Anthropic Messages API client for Claude.
//!
//! Mirrors the public surface of the (legacy) `claude_cli` module so the
//! provider dispatch in `pipeline.rs` / `conflict.rs` / `main.rs` can pick
//! `"claude"` and get the same `KeywordResult` / `String` / `ConflictQueries`
//! / `Vec<ConflictFlag>` return shapes as Gemini.
//!
//! API: https://docs.anthropic.com/en/api/messages

use reqwest::Client;
use serde_json::{json, Value};

use crate::gemini::{
    parse_conflict_flags_json, parse_conflict_queries_json, parse_keyword_json,
    ConflictFlag, ConflictQueries, KeywordResult, Paper,
};

const ANTHROPIC_API: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";

/// Issue a single non-streaming message to the Anthropic API and return the
/// concatenated text content.
async fn claude_call(
    client: &Client,
    api_key: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, String> {
    let body = json!({
        "model": DEFAULT_MODEL,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "messages": [{
            "role": "user",
            "content": prompt,
        }],
    });

    let resp = client
        .post(ANTHROPIC_API)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Claude request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Claude API error {status}: {text}"));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Claude response: {e}"))?;

    extract_text(&data)
}

fn extract_text(data: &Value) -> Result<String, String> {
    let parts = data["content"]
        .as_array()
        .ok_or_else(|| format!("Unexpected Claude response shape: {data}"))?;

    let text: String = parts
        .iter()
        .filter(|p| p["type"].as_str() == Some("text"))
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("");

    if text.is_empty() {
        return Err(format!("Claude returned no text content: {data}"));
    }

    Ok(text.trim().to_string())
}

/// Extract search keywords and a newsletter title from a natural-language query.
pub async fn extract_keywords(
    client: &Client,
    query: &str,
    api_key: &str,
) -> Result<KeywordResult, String> {
    let prompt = format!(
        r#"Convert this research interest into optimal academic search keywords.

User interest: "{query}"

Return ONLY a JSON object with two keys:
  "keywords": a short search string (3-8 words, no special chars)
  "title": a concise newsletter title (3-5 words)

Example: {{"keywords": "large language models drug discovery", "title": "LLMs in Drug Discovery"}}

Respond with valid JSON only, no markdown fences."#
    );

    let text = claude_call(client, api_key, &prompt, 512, 0.2).await?;
    parse_keyword_json(&text, query)
}

/// Curate and summarize papers as a Markdown newsletter.
pub async fn summarize(
    client: &Client,
    papers: &[Paper],
    user_query: &str,
    api_key: &str,
) -> Result<String, String> {
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
    let prompt = format!(
        r#"You are a research newsletter curator. The reader is interested in:
"{user_query}"

Below are {n} papers from academic databases.

Select the 10 most relevant and impactful papers. For each write:
- A clear, jargon-free 2-3 sentence summary (what it does, why it matters)
- A relevance tag (e.g. Machine Learning, Biology, Climate, Economics…)

End with a 2-sentence overview of the theme across these papers.

Format as Markdown: ## for titles linked to paper URL, paragraphs for summaries, **bold** for tags. Do NOT wrap in code fences.

{paper_list}"#
    );

    let mut text = claude_call(client, api_key, &prompt, 8192, 0.7).await?;

    if text.starts_with("```") {
        if let Some(newline) = text.find('\n') {
            text = text[newline + 1..].to_string();
        }
    }
    if text.ends_with("```") {
        text = text[..text.len() - 3].trim_end().to_string();
    }

    Ok(text)
}

/// Build a multi-pronged conflict-search query set from a research description.
pub async fn extract_conflict_queries(
    client: &Client,
    research_description: &str,
    key_terms: &[String],
    api_key: &str,
) -> Result<ConflictQueries, String> {
    let terms_str = key_terms.join(", ");
    let prompt = format!(
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
    );

    let text = claude_call(client, api_key, &prompt, 1024, 0.2).await?;
    parse_conflict_queries_json(&text, research_description, key_terms)
}

/// Score papers for overlap with the researcher's work; return everything ≥ 25.
pub async fn evaluate_conflicts(
    client: &Client,
    papers: &[Paper],
    research_description: &str,
    key_terms: &[String],
    api_key: &str,
) -> Result<Vec<ConflictFlag>, String> {
    if papers.is_empty() {
        return Ok(Vec::new());
    }

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
    let prompt = format!(
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
    );

    let text = claude_call(client, api_key, &prompt, 8192, 0.3).await?;
    parse_conflict_flags_json(&text)
}
