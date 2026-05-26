use crate::gemini::{decode_inverted_index, Paper};
use chrono::{Duration, Utc};
use reqwest::Client;

/// Search OpenAlex for academic articles matching `keywords`.
///
/// - `max_results`: capped at 50 (OpenAlex per-page limit)
/// - `days_back`: only return papers published within this many days
pub async fn search_openalex(
    client: &Client,
    keywords: &str,
    max_results: u32,
    days_back: u32,
) -> Result<Vec<Paper>, String> {
    let from_date = (Utc::now() - Duration::days(days_back as i64))
        .format("%Y-%m-%d")
        .to_string();

    let per_page = max_results.min(50);
    let encoded = urlencoded(keywords);

    let url = format!(
        "https://api.openalex.org/works\
        ?search={encoded}\
        &filter=from_publication_date:{from_date},type:article\
        &sort=relevance_score:desc\
        &per-page={per_page}\
        &select=id,title,abstract_inverted_index,authorships,publication_date,doi,primary_location\
        &mailto=research.newsletter@local"
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("OpenAlex request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAlex API error {status}: {text}"));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("OpenAlex JSON parse error: {e}"))?;

    let results = data["results"].as_array().cloned().unwrap_or_default();

    let papers: Vec<Paper> = results
        .iter()
        .filter_map(|work| {
            let title = work["title"].as_str()?.trim().to_string();
            if title.is_empty() {
                return None;
            }

            let abstract_text = decode_inverted_index(&work["abstract_inverted_index"]);

            let authors: Vec<String> = work["authorships"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .take(3)
                .filter_map(|a| a["author"]["display_name"].as_str())
                .map(|s| s.to_string())
                .collect();

            let loc = &work["primary_location"];
            let url = loc["landing_page_url"]
                .as_str()
                .or_else(|| work["doi"].as_str())
                .or_else(|| work["id"].as_str())
                .unwrap_or("")
                .to_string();

            let date = work["publication_date"]
                .as_str()
                .unwrap_or("")
                .to_string();

            Some(Paper {
                title,
                abstract_text,
                authors: authors.join(", "),
                date,
                url,
                source: "OpenAlex".to_string(),
            })
        })
        .collect();

    Ok(papers)
}

/// Paginated search that fetches multiple pages to get more results.
/// Used by the conflict scanner which needs higher recall.
pub async fn search_openalex_paginated(
    client: &Client,
    keywords: &str,
    total_results: u32,
    days_back: u32,
) -> Result<Vec<Paper>, String> {
    let from_date = (Utc::now() - Duration::days(days_back as i64))
        .format("%Y-%m-%d")
        .to_string();

    let per_page: u32 = 50;
    let max_pages = (total_results / per_page).max(1).min(4); // cap at 4 pages = 200 results
    let encoded = urlencoded(keywords);

    let mut all_papers: Vec<Paper> = Vec::new();

    for page in 1..=max_pages {
        let url = format!(
            "https://api.openalex.org/works\
            ?search={encoded}\
            &filter=from_publication_date:{from_date},type:article\
            &sort=relevance_score:desc\
            &page={page}\
            &per-page={per_page}\
            &select=id,title,abstract_inverted_index,authorships,publication_date,doi,primary_location\
            &mailto=research.newsletter@local"
        );

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("OpenAlex request failed (page {page}): {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAlex API error {status} (page {page}): {text}"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("OpenAlex JSON parse error (page {page}): {e}"))?;

        let results = data["results"].as_array().cloned().unwrap_or_default();

        if results.is_empty() {
            break; // No more results
        }

        let papers: Vec<Paper> = results
            .iter()
            .filter_map(|work| {
                let title = work["title"].as_str()?.trim().to_string();
                if title.is_empty() {
                    return None;
                }

                let abstract_text = decode_inverted_index(&work["abstract_inverted_index"]);

                let authors: Vec<String> = work["authorships"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .take(3)
                    .filter_map(|a| a["author"]["display_name"].as_str())
                    .map(|s| s.to_string())
                    .collect();

                let loc = &work["primary_location"];
                let url = loc["landing_page_url"]
                    .as_str()
                    .or_else(|| work["doi"].as_str())
                    .or_else(|| work["id"].as_str())
                    .unwrap_or("")
                    .to_string();

                let date = work["publication_date"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                Some(Paper {
                    title,
                    abstract_text,
                    authors: authors.join(", "),
                    date,
                    url,
                    source: "OpenAlex".to_string(),
                })
            })
            .collect();

        all_papers.extend(papers);

        // Be polite to the API: small delay between page requests
        if page < max_pages {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    Ok(all_papers)
}

fn urlencoded(s: &str) -> String {
    // Simple percent-encoding for query strings
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b' ' => {
                if b == b' ' {
                    out.push('+');
                } else {
                    out.push(b as char);
                }
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}
