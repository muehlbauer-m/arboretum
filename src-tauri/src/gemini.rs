use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

const GEMINI_BASE: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

/// Call the Gemini API with the given prompt and generation config.
async fn gemini_call(
    client: &Client,
    api_key: &str,
    prompt: &str,
    temperature: f64,
    max_tokens: u32,
    thinking: bool,
) -> Result<Value, String> {
    let mut generation_config = json!({
        "temperature": temperature,
        "maxOutputTokens": max_tokens,
    });

    if !thinking {
        generation_config["thinkingConfig"] = json!({ "thinkingBudget": 0 });
    }

    let body = json!({
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": generation_config,
    });

    let url = format!("{GEMINI_BASE}?key={api_key}");

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini API error {status}: {text}"));
    }

    resp.json::<Value>()
        .await
        .map_err(|e| format!("Failed to parse Gemini response: {e}"))
}

/// Use Gemini to extract search keywords and a newsletter title from a
/// natural-language research query.
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

    let data = gemini_call(client, api_key, &prompt, 0.2, 256, false).await?;
    let raw_text = extract_text_from_response(&data)?;
    parse_keyword_json(&raw_text, query)
}

pub struct KeywordResult {
    pub keywords: String,
    pub title: String,
}

pub(crate) fn parse_keyword_json(text: &str, fallback_query: &str) -> Result<KeywordResult, String> {
    // Strip markdown fences
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parse
    if let Ok(v) = serde_json::from_str::<Value>(cleaned) {
        if let (Some(kw), Some(title)) = (
            v["keywords"].as_str(),
            v["title"].as_str(),
        ) {
            return Ok(KeywordResult {
                keywords: kw.to_string(),
                title: title.to_string(),
            });
        }
    }

    // Try to find first {...} block
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            let slice = &cleaned[start..=end];
            if let Ok(v) = serde_json::from_str::<Value>(slice) {
                if let (Some(kw), Some(title)) = (v["keywords"].as_str(), v["title"].as_str()) {
                    return Ok(KeywordResult {
                        keywords: kw.to_string(),
                        title: title.to_string(),
                    });
                }
            }
        }
    }

    // Fallback: derive from query
    let words: Vec<&str> = fallback_query.split_whitespace().take(8).collect();
    let keywords = words.join(" ").to_lowercase();
    let title_words: Vec<String> = fallback_query
        .split_whitespace()
        .take(4)
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect();
    Ok(KeywordResult {
        keywords,
        title: title_words.join(" "),
    })
}

#[derive(Debug, Clone)]
pub struct Paper {
    pub title: String,
    pub abstract_text: String,
    pub authors: String,
    pub date: String,
    pub url: String,
    pub source: String,
}

/// Use Gemini to pick the 10 most relevant papers and format them as Markdown.
pub async fn summarize_with_gemini(
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

    let data = gemini_call(client, api_key, &prompt, 0.7, 65536, false).await?;
    let finish_reason = data["candidates"][0]["finishReason"]
        .as_str()
        .unwrap_or("UNKNOWN")
        .to_string();

    let parts = data["candidates"][0]["content"]["parts"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut text: String = parts
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("");

    // Strip any code fences Gemini might have added
    if text.starts_with("```") {
        if let Some(newline) = text.find('\n') {
            text = text[newline + 1..].to_string();
        }
    }
    if text.ends_with("```") {
        text = text[..text.len() - 3].trim_end().to_string();
    }

    match finish_reason.as_str() {
        "STOP" | "MAX_TOKENS" => {}
        other => {
            text.push_str(&format!(
                "\n\n*(Response ended early — finishReason: {other})*"
            ));
        }
    }

    if finish_reason == "MAX_TOKENS" {
        text.push_str(
            "\n\n*(Note: response was capped at the token limit — try reducing Max papers)*",
        );
    }

    Ok(text)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictQueries {
    pub primary: String,
    pub method_queries: Vec<String>,
    pub broad_query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFlag {
    pub paper_index: usize,
    pub score: u32,
    pub title: String,
    pub url: String,
    pub authors: String,
    pub date: String,
    pub summary: String,
    pub overlap: String,
    pub difference: String,
    pub threat_level: String,
    pub matched_terms: Vec<String>,
}

/// Use Gemini to generate multiple search queries from a research description
/// and key terms for exhaustive conflict scanning.
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

    let data = gemini_call(client, api_key, &prompt, 0.2, 512, false).await?;
    let raw_text = extract_text_from_response(&data)?;
    parse_conflict_queries_json(&raw_text, research_description, key_terms)
}

pub(crate) fn parse_conflict_queries_json(
    text: &str,
    research_description: &str,
    key_terms: &[String],
) -> Result<ConflictQueries, String> {
    // Strip markdown fences
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parse
    if let Ok(v) = serde_json::from_str::<Value>(cleaned) {
        if let Some(q) = extract_conflict_queries_from_value(&v) {
            return Ok(q);
        }
    }

    // Try to find first {...} block
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            let slice = &cleaned[start..=end];
            if let Ok(v) = serde_json::from_str::<Value>(slice) {
                if let Some(q) = extract_conflict_queries_from_value(&v) {
                    return Ok(q);
                }
            }
        }
    }

    // Fallback: derive from research description and key terms
    let words: Vec<&str> = research_description.split_whitespace().take(6).collect();
    let primary = words.join(" ").to_lowercase();
    let broad_query = if key_terms.is_empty() {
        research_description
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    } else {
        key_terms[0].clone()
    };
    let method_queries: Vec<String> = key_terms
        .iter()
        .take(3)
        .cloned()
        .collect();

    Ok(ConflictQueries {
        primary,
        method_queries,
        broad_query,
    })
}

fn extract_conflict_queries_from_value(v: &Value) -> Option<ConflictQueries> {
    let primary = v["primary"].as_str()?.to_string();
    let broad_query = v["broad_query"].as_str()?.to_string();
    let method_queries: Vec<String> = v["method_queries"]
        .as_array()?
        .iter()
        .filter_map(|q| q.as_str().map(|s| s.to_string()))
        .collect();
    Some(ConflictQueries {
        primary,
        method_queries,
        broad_query,
    })
}

/// Use Gemini to evaluate papers for overlap with the researcher's work.
/// Scores each paper 0-100 on competition overlap and returns structured results
/// for papers scoring 25+.
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

    let data = gemini_call(client, api_key, &prompt, 0.3, 65536, false).await?;

    let parts = data["candidates"][0]["content"]["parts"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let raw_text: String = parts
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("");

    parse_conflict_flags_json(&raw_text)
}

pub(crate) fn parse_conflict_flags_json(text: &str) -> Result<Vec<ConflictFlag>, String> {
    // Strip markdown fences
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parse as array
    if let Ok(arr) = serde_json::from_str::<Vec<Value>>(cleaned) {
        return Ok(parse_conflict_flags_from_array(&arr));
    }

    // Try to find JSON array [...] in the text
    if let Some(start) = cleaned.find('[') {
        if let Some(end) = cleaned.rfind(']') {
            let slice = &cleaned[start..=end];
            if let Ok(arr) = serde_json::from_str::<Vec<Value>>(slice) {
                return Ok(parse_conflict_flags_from_array(&arr));
            }
        }
    }

    // Try to find a single {...} object and wrap it
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            let slice = &cleaned[start..=end];
            if let Ok(v) = serde_json::from_str::<Value>(slice) {
                if let Some(flag) = parse_single_conflict_flag(&v) {
                    return Ok(vec![flag]);
                }
            }
        }
    }

    // No valid JSON found, return empty
    Ok(Vec::new())
}

fn parse_conflict_flags_from_array(arr: &[Value]) -> Vec<ConflictFlag> {
    arr.iter().filter_map(parse_single_conflict_flag).collect()
}

fn parse_single_conflict_flag(v: &Value) -> Option<ConflictFlag> {
    let paper_index = v["paper_index"].as_u64()? as usize;
    let score = v["score"].as_u64().unwrap_or(0) as u32;

    Some(ConflictFlag {
        paper_index,
        score,
        title: v["title"].as_str().unwrap_or("").to_string(),
        url: v["url"].as_str().unwrap_or("").to_string(),
        authors: v["authors"].as_str().unwrap_or("").to_string(),
        date: v["date"].as_str().unwrap_or("").to_string(),
        summary: v["summary"].as_str().unwrap_or("").to_string(),
        overlap: v["overlap"].as_str().unwrap_or("").to_string(),
        difference: v["difference"].as_str().unwrap_or("").to_string(),
        threat_level: v["threat_level"].as_str().unwrap_or("low").to_string(),
        matched_terms: v["matched_terms"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn extract_text_from_response(data: &Value) -> Result<String, String> {
    data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            format!(
                "Unexpected Gemini response structure: {}",
                serde_json::to_string_pretty(data).unwrap_or_default()
            )
        })
}

/// Reconstruct plaintext from OpenAlex inverted index.
/// The inverted index is a HashMap<word, Vec<position>>.
pub fn decode_inverted_index(inv_idx: &Value) -> String {
    if inv_idx.is_null() {
        return String::new();
    }

    let obj = match inv_idx.as_object() {
        Some(o) => o,
        None => return String::new(),
    };

    let mut pos_word: HashMap<u32, &str> = HashMap::new();
    for (word, positions) in obj {
        if let Some(arr) = positions.as_array() {
            for pos in arr {
                if let Some(p) = pos.as_u64() {
                    pos_word.insert(p as u32, word.as_str());
                }
            }
        }
    }

    let mut sorted_positions: Vec<u32> = pos_word.keys().copied().collect();
    sorted_positions.sort_unstable();

    sorted_positions
        .iter()
        .filter_map(|p| pos_word.get(p).copied())
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_keyword_json_valid() {
        let text = r#"{"keywords": "machine learning optimization", "title": "ML Research"}"#;
        let result = parse_keyword_json(text, "fallback").unwrap();
        assert_eq!(result.keywords, "machine learning optimization");
        assert_eq!(result.title, "ML Research");
    }

    #[test]
    fn test_parse_keyword_json_with_fences() {
        let text = "```json\n{\"keywords\": \"small area estimation\", \"title\": \"SAE Methods\"}\n```";
        let result = parse_keyword_json(text, "fallback").unwrap();
        assert_eq!(result.keywords, "small area estimation");
    }

    #[test]
    fn test_parse_keyword_json_embedded_braces() {
        let text = "Here is the result: {\"keywords\": \"bayesian models\", \"title\": \"Bayesian\"} done.";
        let result = parse_keyword_json(text, "fallback").unwrap();
        assert_eq!(result.keywords, "bayesian models");
    }

    #[test]
    fn test_parse_keyword_json_fallback() {
        let text = "this is not json at all!!!";
        let result = parse_keyword_json(text, "small area estimation fay herriot").unwrap();
        // fallback derives from query
        assert!(!result.keywords.is_empty());
        assert!(!result.title.is_empty());
    }

    #[test]
    fn test_decode_inverted_index_basic() {
        let inv = json!({
            "hello": [0, 2],
            "world": [1],
            "foo": [3]
        });
        let result = decode_inverted_index(&inv);
        assert_eq!(result, "hello world hello foo");
    }

    #[test]
    fn test_decode_inverted_index_null() {
        let result = decode_inverted_index(&serde_json::Value::Null);
        assert_eq!(result, "");
    }

    #[test]
    fn test_decode_inverted_index_empty() {
        let result = decode_inverted_index(&json!({}));
        assert_eq!(result, "");
    }
}
