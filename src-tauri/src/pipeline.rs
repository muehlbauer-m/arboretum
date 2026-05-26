use crate::config::AppConfig;
use crate::gemini::{extract_keywords, summarize_with_gemini, Paper};
use crate::claude_api;
use crate::local_llm;
use crate::sources::{arxiv::search_arxiv, openalex::search_openalex};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;

/// A single topic sent from the frontend (matches config::TopicRequest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicRequest {
    pub query: String,
}

/// Result returned to the frontend for each topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
    pub topic: String,
    pub title: String,
    pub path: String,
    pub error: Option<String>,
}

/// Progress payload emitted via Tauri events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub topic: String,
    pub step: String,
    pub message: String,
    pub done: bool,
    pub error: bool,
}

const PROGRESS_EVENT: &str = "newsletter-progress";

/// Run the full generation pipeline for a single topic.
pub async fn run_single_topic(
    app: tauri::AppHandle,
    client: Client,
    topic: TopicRequest,
    sources: Vec<String>,
    max_papers: u32,
    days_back: u32,
    config: AppConfig,
) -> GenerateResult {
    let query = topic.query.clone();

    macro_rules! emit {
        ($step:expr, $msg:expr) => {
            if let Err(e) = app.emit(
                PROGRESS_EVENT,
                ProgressEvent {
                    topic: query.clone(),
                    step: $step.to_string(),
                    message: $msg.to_string(),
                    done: false,
                    error: false,
                },
            ) {
                eprintln!("[newsletter] Failed to emit progress event: {e}");
            }
        };
    }

    macro_rules! emit_err {
        ($step:expr, $msg:expr) => {
            if let Err(e) = app.emit(
                PROGRESS_EVENT,
                ProgressEvent {
                    topic: query.clone(),
                    step: $step.to_string(),
                    message: $msg.to_string(),
                    done: false,
                    error: true,
                },
            ) {
                eprintln!("[newsletter] Failed to emit progress event: {e}");
            }
        };
    }

    macro_rules! emit_done {
        ($msg:expr) => {
            if let Err(e) = app.emit(
                PROGRESS_EVENT,
                ProgressEvent {
                    topic: query.clone(),
                    step: "done".to_string(),
                    message: $msg.to_string(),
                    done: true,
                    error: false,
                },
            ) {
                eprintln!("[newsletter] Failed to emit progress event: {e}");
            }
        };
    }

    // ── Step 1: extract keywords ──────────────────────────────────────────────
    let provider = config.ai_provider.as_str();
    let kw_label = match provider {
        "claude" => "Asking Claude to extract search keywords…",
        "local"  => "Asking the local model to extract search keywords…",
        _        => "Asking Gemini to extract search keywords…",
    };
    emit!("keywords", kw_label);
    let kw_result = match match provider {
        "claude" => claude_api::extract_keywords(&client, &query, &config.claude_api_key).await,
        "local"  => local_llm::extract_keywords(&client, &config.local_llm, &query).await,
        _        => extract_keywords(&client, &query, &config.gemini_api_key).await,
    } {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("Keyword extraction failed: {e}");
            emit_err!("keywords", &msg);
            return GenerateResult {
                topic: query,
                title: String::new(),
                path: String::new(),
                error: Some(msg),
            };
        }
    };
    emit!(
        "keywords",
        format!(
            "Keywords: \"{}\"  |  Title: \"{}\"",
            kw_result.keywords, kw_result.title
        )
    );

    // ── Step 2: fetch papers ──────────────────────────────────────────────────
    let mut all_papers: Vec<Paper> = Vec::new();
    let per_source = if sources.is_empty() {
        max_papers
    } else {
        (max_papers / sources.len() as u32).max(10)
    };

    if sources.contains(&"openalex".to_string()) {
        emit!(
            "openalex",
            format!("Searching OpenAlex (last {days_back} days)…")
        );
        match search_openalex(&client, &kw_result.keywords, per_source, days_back).await {
            Ok(papers) => {
                let n = papers.len();
                all_papers.extend(papers);
                emit!("openalex", format!("→ {n} papers from OpenAlex"));
            }
            Err(e) => {
                emit_err!("openalex", format!("OpenAlex search failed: {e}"));
            }
        }
    }

    if sources.contains(&"arxiv".to_string()) {
        emit!("arxiv", "Searching arXiv…");
        match search_arxiv(&client, &kw_result.keywords, per_source).await {
            Ok(papers) => {
                let n = papers.len();
                all_papers.extend(papers);
                emit!("arxiv", format!("→ {n} papers from arXiv"));
            }
            Err(e) => {
                emit_err!("arxiv", format!("arXiv search failed: {e}"));
            }
        }
    }

    if all_papers.is_empty() {
        let msg = "No papers found. Try broader or different keywords.".to_string();
        emit_err!("search", &msg);
        return GenerateResult {
            topic: query,
            title: kw_result.title,
            path: String::new(),
            error: Some(msg),
        };
    }

    let summarize_label = match provider {
        "claude" => format!("Sending {} papers to Claude for curation…", all_papers.len()),
        "local"  => format!(
            "Sending {} papers to local model ({}) for curation… expect a few minutes.",
            all_papers.len(),
            config.local_llm.model
        ),
        _        => format!("Sending {} papers to Gemini for curation…", all_papers.len()),
    };
    emit!("summarize", summarize_label);

    // ── Step 3: summarize ─────────────────────────────────────────────────────
    let summary = match match provider {
        "claude" => claude_api::summarize(&client, &all_papers, &query, &config.claude_api_key).await,
        "local"  => local_llm::summarize(Some(&app), &client, &config.local_llm, &all_papers, &query).await,
        _        => summarize_with_gemini(&client, &all_papers, &query, &config.gemini_api_key).await,
    } {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("Summarization failed: {e}");
                emit_err!("summarize", &msg);
                return GenerateResult {
                    topic: query,
                    title: kw_result.title,
                    path: String::new(),
                    error: Some(msg),
                };
            }
        };

    // ── Step 4: save ──────────────────────────────────────────────────────────
    emit!("save", "Saving newsletter…");
    match save_newsletter(&summary, &kw_result.title, &config.output_dir, config.ai_provider.as_str()) {
        Ok(path) => {
            emit_done!(format!("Saved to: {path}"));
            GenerateResult {
                topic: query,
                title: kw_result.title,
                path,
                error: None,
            }
        }
        Err(e) => {
            let msg = format!("Failed to save newsletter: {e}");
            emit_err!("save", &msg);
            GenerateResult {
                topic: query,
                title: kw_result.title,
                path: String::new(),
                error: Some(msg),
            }
        }
    }
}

/// Write the newsletter Markdown to disk.
pub fn save_newsletter(content: &str, title: &str, output_dir: &str, ai_provider: &str) -> Result<String, String> {
    let today = Utc::now();
    let date_str = today.format("%Y-%m-%d").to_string();
    let date_readable = today.format("%A, %B %d, %Y").to_string();

    let provider_display = match ai_provider {
        "claude" => "Claude",
        "gemini" => "Gemini",
        "local" => "a local model (Ollama)",
        other => other,
    };

    let markdown = format!(
        "# {title}\n*{date_readable}*\n\n---\n\n{content}\n\n---\n\n*Generated automatically via OpenAlex & arXiv using {provider_display}.*\n"
    );

    let dir = Path::new(output_dir);
    fs::create_dir_all(dir).map_err(|e| format!("Cannot create output dir: {e}"))?;

    // Find a filename that doesn't already exist to avoid overwriting
    let base = dir.join(format!("newsletter-{date_str}.md"));
    let filename = if !base.exists() {
        base
    } else {
        let mut counter = 2u32;
        loop {
            let candidate = dir.join(format!("newsletter-{date_str}-{counter}.md"));
            if !candidate.exists() {
                break candidate;
            }
            counter += 1;
        }
    };

    fs::write(&filename, &markdown).map_err(|e| format!("Cannot write file: {e}"))?;

    Ok(filename.to_string_lossy().to_string())
}

/// Metadata for a saved newsletter (used by the History page).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsletterMeta {
    pub path: String,
    pub filename: String,
    pub title: String,
    pub date: String,
    pub size_kb: u64,
}

/// Scan `output_dir` for `.md` files and return their metadata, sorted newest first.
pub fn list_newsletters(output_dir: &str) -> Vec<NewsletterMeta> {
    let dir = Path::new(output_dir);
    if !dir.is_dir() {
        return Vec::new();
    }

    let mut items: Vec<NewsletterMeta> = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let size_kb = entry.metadata().map(|m| m.len() / 1024).unwrap_or(0);

        // Parse date from filename: newsletter-YYYY-MM-DD.md
        let date = extract_date_from_filename(&filename);

        // Read first line for title
        let title = read_first_heading(&path).unwrap_or_else(|| filename.clone());

        items.push(NewsletterMeta {
            path: path.to_string_lossy().to_string(),
            filename,
            title,
            date,
            size_kb,
        });
    }

    // Sort by date descending
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items
}

pub(crate) fn extract_date_from_filename(filename: &str) -> String {
    // Expects: newsletter-YYYY-MM-DD.md
    let stem = filename.trim_end_matches(".md");
    if stem.starts_with("newsletter-") {
        stem.trim_start_matches("newsletter-").to_string()
    } else {
        stem.to_string()
    }
}

fn read_first_heading(path: &PathBuf) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let trimmed = line.trim_start_matches('#').trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_extract_date_from_filename() {
        assert_eq!(extract_date_from_filename("newsletter-2024-01-15.md"), "2024-01-15");
        assert_eq!(extract_date_from_filename("newsletter-2026-03-25.md"), "2026-03-25");
        assert_eq!(extract_date_from_filename("other-file.md"), "other-file");
    }

    #[test]
    fn test_save_newsletter_creates_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_str().unwrap();

        let result = save_newsletter("## Test Content", "Test Title", dir, "gemini");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(Path::new(&path).exists());

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("# Test Title"));
        assert!(contents.contains("## Test Content"));
        assert!(contents.contains("using Gemini."));
    }

    #[test]
    fn test_list_newsletters_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let items = list_newsletters(tmp.path().to_str().unwrap());
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_list_newsletters_finds_md_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_str().unwrap();

        // Create some newsletter files
        std::fs::write(
            tmp.path().join("newsletter-2024-01-15.md"),
            "# Test Newsletter\n*January 15, 2024*\n\nContent here."
        ).unwrap();
        std::fs::write(
            tmp.path().join("newsletter-2024-01-20.md"),
            "# Second Newsletter\n*January 20, 2024*\n\nMore content."
        ).unwrap();

        let items = list_newsletters(dir);
        assert_eq!(items.len(), 2);
        // Sorted newest first
        assert_eq!(items[0].date, "2024-01-20");
        assert_eq!(items[1].date, "2024-01-15");
    }
}
