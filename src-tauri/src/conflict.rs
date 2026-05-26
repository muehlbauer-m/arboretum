use crate::claude_api;
use crate::config::{AppConfig, CompetitionProfile};
use crate::gemini::{self, ConflictFlag, ConflictQueries, Paper};
use crate::local_llm;
use crate::pipeline::ProgressEvent;
use crate::sources::{arxiv::search_arxiv, openalex::search_openalex_paginated};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::Emitter;

const CONFLICT_EVENT: &str = "conflict-progress";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlapResult {
    pub paper_title: String,
    pub paper_authors: String,
    pub paper_url: String,
    pub paper_date: String,
    pub paper_source: String,
    pub overlap_score: u32,
    pub overlap_explanation: String,
    pub matched_terms: Vec<String>,
    pub profile_id: String,
    pub profile_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictScanResult {
    pub profile_id: String,
    pub profile_name: String,
    pub overlaps: Vec<OverlapResult>,
    pub scanned_at: String,
    pub papers_checked: usize,
    pub error: Option<String>,
}

pub async fn run_conflict_scan(
    app: tauri::AppHandle,
    client: Client,
    profile: CompetitionProfile,
    sources: Vec<String>,
    max_papers: u32,
    days_back: u32,
    config: AppConfig,
) -> ConflictScanResult {
    let profile_id = profile.id.clone();
    let profile_name = profile.name.clone();

    macro_rules! emit {
        ($step:expr, $msg:expr) => {
            if let Err(e) = app.emit(
                CONFLICT_EVENT,
                ProgressEvent {
                    topic: profile_name.clone(),
                    step: $step.to_string(),
                    message: $msg.to_string(),
                    done: false,
                    error: false,
                },
            ) {
                eprintln!("[conflict] Failed to emit progress event: {e}");
            }
        };
    }

    macro_rules! emit_err {
        ($step:expr, $msg:expr) => {
            if let Err(e) = app.emit(
                CONFLICT_EVENT,
                ProgressEvent {
                    topic: profile_name.clone(),
                    step: $step.to_string(),
                    message: $msg.to_string(),
                    done: false,
                    error: true,
                },
            ) {
                eprintln!("[conflict] Failed to emit progress event: {e}");
            }
        };
    }

    macro_rules! emit_done {
        ($msg:expr) => {
            if let Err(e) = app.emit(
                CONFLICT_EVENT,
                ProgressEvent {
                    topic: profile_name.clone(),
                    step: "done".to_string(),
                    message: $msg.to_string(),
                    done: true,
                    error: false,
                },
            ) {
                eprintln!("[conflict] Failed to emit progress event: {e}");
            }
        };
    }

    // ── Step 1: Extract conflict queries ─────────────────────────────────────
    let provider = config.ai_provider.as_str();
    let query_label = match provider {
        "claude" => "Asking Claude to generate search queries...",
        "local"  => "Asking the local model to generate search queries...",
        _        => "Asking Gemini to generate search queries...",
    };
    emit!("queries", query_label);

    let queries: ConflictQueries = match match provider {
        "claude" => claude_api::extract_conflict_queries(
            &client,
            &profile.research_description,
            &profile.key_terms,
            &config.claude_api_key,
        ).await,
        "local" => local_llm::extract_conflict_queries(
            &client,
            &config.local_llm,
            &profile.research_description,
            &profile.key_terms,
        ).await,
        _ => gemini::extract_conflict_queries(
            &client,
            &profile.research_description,
            &profile.key_terms,
            &config.gemini_api_key,
        ).await,
    } {
        Ok(q) => {
            emit!(
                "queries",
                format!(
                    "Generated queries: primary=\"{}\", {} method queries, broad=\"{}\"",
                    q.primary,
                    q.method_queries.len(),
                    q.broad_query
                )
            );
            q
        }
        Err(e) => {
            let msg = format!("Failed to extract conflict queries: {e}");
            emit_err!("queries", &msg);
            return ConflictScanResult {
                profile_id,
                profile_name,
                overlaps: Vec::new(),
                scanned_at: chrono::Utc::now().to_rfc3339(),
                papers_checked: 0,
                error: Some(msg),
            };
        }
    };

    // ── Step 2: Multi-round search ───────────────────────────────────────────
    // Build the list of all queries to run
    let mut all_queries: Vec<(String, String)> = Vec::new(); // (label, query_string)
    all_queries.push(("primary".to_string(), queries.primary.clone()));
    for (i, mq) in queries.method_queries.iter().enumerate() {
        all_queries.push((format!("method-{}", i + 1), mq.clone()));
    }
    all_queries.push(("broad".to_string(), queries.broad_query.clone()));

    // Track paper source for each paper
    let mut all_papers: Vec<Paper> = Vec::new();

    let per_source_per_query = (max_papers / all_queries.len().max(1) as u32).max(50);

    for (label, query_str) in &all_queries {
        emit!(
            "search",
            format!("Searching for \"{}\" ({} query)...", query_str, label)
        );

        // Search OpenAlex (paginated)
        if sources.contains(&"openalex".to_string()) {
            match search_openalex_paginated(&client, query_str, per_source_per_query, days_back)
                .await
            {
                Ok(papers) => {
                    let n = papers.len();
                    all_papers.extend(papers);
                    emit!(
                        "search",
                        format!("  OpenAlex ({label}): {n} papers")
                    );
                }
                Err(e) => {
                    emit_err!(
                        "search",
                        format!("  OpenAlex ({label}) failed: {e}")
                    );
                    // Continue with other searches
                }
            }
        }

        // Search arXiv
        if sources.contains(&"arxiv".to_string()) {
            match search_arxiv(&client, query_str, per_source_per_query.min(100)).await {
                Ok(papers) => {
                    let n = papers.len();
                    all_papers.extend(papers);
                    emit!(
                        "search",
                        format!("  arXiv ({label}): {n} papers")
                    );
                }
                Err(e) => {
                    emit_err!(
                        "search",
                        format!("  arXiv ({label}) failed: {e}")
                    );
                    // Continue with other searches
                }
            }
        }
    }

    if all_papers.is_empty() {
        let msg = "No papers found across all search queries.".to_string();
        emit_err!("search", &msg);
        return ConflictScanResult {
            profile_id,
            profile_name,
            overlaps: Vec::new(),
            scanned_at: chrono::Utc::now().to_rfc3339(),
            papers_checked: 0,
            error: Some(msg),
        };
    }

    // ── Step 3: Deduplication ────────────────────────────────────────────────
    emit!(
        "dedup",
        format!("Deduplicating {} papers...", all_papers.len())
    );

    let before_count = all_papers.len();
    let deduped = deduplicate_papers(all_papers);
    let after_count = deduped.len();

    emit!(
        "dedup",
        format!(
            "Deduplicated: {} -> {} unique papers",
            before_count, after_count
        )
    );

    // ── Step 4: AI evaluation ────────────────────────────────────────────────
    let eval_label = match provider {
        "claude" => format!("Sending {} papers to Claude for conflict evaluation...", after_count),
        "local"  => format!(
            "Sending {} papers to local model ({}) for conflict evaluation… expect a long wait on this hardware.",
            after_count, config.local_llm.model
        ),
        _        => format!("Sending {} papers to Gemini for conflict evaluation...", after_count),
    };
    emit!("evaluate", eval_label);

    let flags: Vec<ConflictFlag> = match match provider {
        "claude" => claude_api::evaluate_conflicts(
            &client,
            &deduped,
            &profile.research_description,
            &profile.key_terms,
            &config.claude_api_key,
        ).await,
        "local" => local_llm::evaluate_conflicts(
            Some(&app),
            &client,
            &config.local_llm,
            &deduped,
            &profile.research_description,
            &profile.key_terms,
        ).await,
        _ => gemini::evaluate_conflicts(
            &client,
            &deduped,
            &profile.research_description,
            &profile.key_terms,
            &config.gemini_api_key,
        ).await,
    } {
        Ok(f) => {
            emit!(
                "evaluate",
                format!("AI returned {} potential overlaps", f.len())
            );
            f
        }
        Err(e) => {
            let msg = format!("Conflict evaluation failed: {e}");
            emit_err!("evaluate", &msg);
            return ConflictScanResult {
                profile_id,
                profile_name,
                overlaps: Vec::new(),
                scanned_at: chrono::Utc::now().to_rfc3339(),
                papers_checked: after_count,
                error: Some(msg),
            };
        }
    };

    // ── Step 5: Filter and map ───────────────────────────────────────────────
    let threshold = config.conflict_settings.competition_threshold;
    emit!(
        "filter",
        format!("Filtering results with threshold >= {threshold}...")
    );

    let overlaps: Vec<OverlapResult> = flags
        .into_iter()
        .filter(|f| f.score >= threshold)
        .filter_map(|f| {
            // paper_index is 1-based from the AI; convert to 0-based for lookup
            let idx = f.paper_index.checked_sub(1)?;
            let paper = deduped.get(idx);

            // Determine source from the paper if available, otherwise use flag data
            let paper_source = paper
                .map(|p| p.source.clone())
                .unwrap_or_else(|| "unknown".to_string());

            Some(OverlapResult {
                paper_title: f.title,
                paper_authors: f.authors,
                paper_url: f.url,
                paper_date: f.date,
                paper_source,
                overlap_score: f.score,
                overlap_explanation: f.overlap,
                matched_terms: f.matched_terms,
                profile_id: profile_id.clone(),
                profile_name: profile_name.clone(),
            })
        })
        .collect();

    emit_done!(format!(
        "Scan complete: {} overlaps found (threshold: {threshold})",
        overlaps.len()
    ));

    ConflictScanResult {
        profile_id,
        profile_name,
        overlaps,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        papers_checked: after_count,
        error: None,
    }
}

/// Remove duplicate papers by DOI (if available) or normalized title.
fn deduplicate_papers(papers: Vec<Paper>) -> Vec<Paper> {
    let mut seen_dois: HashSet<String> = HashSet::new();
    let mut seen_titles: HashSet<String> = HashSet::new();
    let mut unique: Vec<Paper> = Vec::new();

    for paper in papers {
        // Try DOI-based dedup first: extract DOI from URL if it looks like one
        let doi = extract_doi(&paper.url);
        if let Some(ref d) = doi {
            if !seen_dois.insert(d.clone()) {
                continue; // Already seen this DOI
            }
        }

        // Title-based dedup
        let normalized = normalize_title(&paper.title);
        if !normalized.is_empty() && !seen_titles.insert(normalized) {
            continue; // Already seen this title
        }

        unique.push(paper);
    }

    unique
}

/// Extract a DOI string from a URL, if present.
fn extract_doi(url: &str) -> Option<String> {
    // DOIs typically look like: https://doi.org/10.xxxx/yyyy or contain "10.xxxx/"
    if let Some(pos) = url.find("10.") {
        let doi_part = &url[pos..];
        // Take until whitespace or end
        let doi = doi_part
            .split_whitespace()
            .next()
            .unwrap_or(doi_part)
            .trim_end_matches('/')
            .to_lowercase();
        if doi.len() > 5 {
            return Some(doi);
        }
    }
    None
}

/// Normalize a title for dedup comparison: lowercase, remove punctuation, collapse spaces.
fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_title() {
        assert_eq!(
            normalize_title("A Novel Approach: Deep Learning for NLP!"),
            "a novel approach deep learning for nlp"
        );
        assert_eq!(
            normalize_title("  Spaces   Everywhere  "),
            "spaces everywhere"
        );
    }

    #[test]
    fn test_extract_doi() {
        assert_eq!(
            extract_doi("https://doi.org/10.1234/abcde"),
            Some("10.1234/abcde".to_string())
        );
        assert_eq!(
            extract_doi("https://example.com/paper/10.5678/xyz123/"),
            Some("10.5678/xyz123".to_string())
        );
        assert_eq!(extract_doi("https://arxiv.org/abs/2401.00001"), None);
    }

    #[test]
    fn test_deduplicate_papers_by_title() {
        let papers = vec![
            Paper {
                title: "Deep Learning for NLP".to_string(),
                abstract_text: "Abstract 1".to_string(),
                authors: "Alice".to_string(),
                date: "2024-01-01".to_string(),
                url: "https://example.com/1".to_string(),
                source: "OpenAlex".to_string(),
            },
            Paper {
                title: "deep learning for nlp".to_string(),
                abstract_text: "Abstract 2".to_string(),
                authors: "Bob".to_string(),
                date: "2024-01-02".to_string(),
                url: "https://example.com/2".to_string(),
                source: "arXiv".to_string(),
            },
            Paper {
                title: "Something Entirely Different".to_string(),
                abstract_text: "Abstract 3".to_string(),
                authors: "Charlie".to_string(),
                date: "2024-01-03".to_string(),
                url: "https://example.com/3".to_string(),
                source: "OpenAlex".to_string(),
            },
        ];
        let deduped = deduplicate_papers(papers);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].title, "Deep Learning for NLP");
        assert_eq!(deduped[1].title, "Something Entirely Different");
    }

    #[test]
    fn test_deduplicate_papers_by_doi() {
        let papers = vec![
            Paper {
                title: "Paper One".to_string(),
                abstract_text: "Abstract 1".to_string(),
                authors: "Alice".to_string(),
                date: "2024-01-01".to_string(),
                url: "https://doi.org/10.1234/paper1".to_string(),
                source: "OpenAlex".to_string(),
            },
            Paper {
                title: "Paper One (different title)".to_string(),
                abstract_text: "Abstract 2".to_string(),
                authors: "Alice".to_string(),
                date: "2024-01-01".to_string(),
                url: "https://doi.org/10.1234/paper1".to_string(),
                source: "arXiv".to_string(),
            },
        ];
        let deduped = deduplicate_papers(papers);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].source, "OpenAlex"); // first one kept
    }

    #[test]
    fn test_deduplicate_empty() {
        let papers: Vec<Paper> = Vec::new();
        let deduped = deduplicate_papers(papers);
        assert!(deduped.is_empty());
    }
}
