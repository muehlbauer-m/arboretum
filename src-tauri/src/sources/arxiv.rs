use crate::gemini::Paper;
use regex::Regex;
use reqwest::Client;

/// Search arXiv for preprints matching `keywords`.
pub async fn search_arxiv(
    client: &Client,
    keywords: &str,
    max_results: u32,
) -> Result<Vec<Paper>, String> {
    // arXiv uses URL-safe query; encode only special chars
    let encoded = urlencoded_arxiv(keywords);
    let url = format!(
        "https://export.arxiv.org/api/query\
        ?search_query=all:{encoded}\
        &start=0\
        &max_results={max_results}\
        &sortBy=submittedDate\
        &sortOrder=descending"
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("arXiv request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("arXiv API error {status}: {text}"));
    }

    let xml = resp
        .text()
        .await
        .map_err(|e| format!("arXiv text read error: {e}"))?;

    Ok(parse_arxiv_xml(&xml))
}

pub(crate) fn parse_arxiv_xml(xml: &str) -> Vec<Paper> {
    let entry_re = match Regex::new(r"(?s)<entry>(.*?)</entry>") {
        Ok(re) => re,
        Err(e) => {
            eprintln!("[newsletter] arxiv regex error: {e}");
            return Vec::new();
        }
    };
    let tag_re =
        |tag: &str, text: &str| -> String {
            let pattern = format!(r"(?s)<{tag}[^>]*>(.*?)</{tag}>");
            if let Ok(re) = Regex::new(&pattern) {
                if let Some(cap) = re.captures(text) {
                    return cap[1].trim().to_string();
                }
            }
            String::new()
        };

    let author_re = match Regex::new(r"(?s)<author>\s*<name>(.*?)</name>") {
        Ok(re) => re,
        Err(e) => {
            eprintln!("[newsletter] arxiv author regex error: {e}");
            return Vec::new();
        }
    };

    let mut papers: Vec<Paper> = Vec::new();

    for cap in entry_re.captures_iter(xml) {
        let entry = &cap[1];

        let title = normalize_whitespace(&tag_re("title", entry));
        if title.is_empty() {
            continue;
        }

        let abstract_text = normalize_whitespace(&tag_re("summary", entry));
        let date = tag_re("published", entry);
        let url = tag_re("id", entry);

        let authors: Vec<String> = author_re
            .captures_iter(entry)
            .take(3)
            .map(|m| m[1].trim().to_string())
            .collect();

        papers.push(Paper {
            title,
            abstract_text,
            authors: authors.join(", "),
            date,
            url,
            source: "arXiv".to_string(),
        });
    }

    papers
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn urlencoded_arxiv(s: &str) -> String {
    // arXiv search API accepts most word characters; encode spaces as +
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arxiv_xml_single_entry() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <feed>
          <entry>
            <id>http://arxiv.org/abs/2401.00001v1</id>
            <title>Test Paper Title</title>
            <summary>This is the abstract text.</summary>
            <published>2024-01-15T00:00:00Z</published>
            <author><name>Alice Smith</name></author>
            <author><name>Bob Jones</name></author>
          </entry>
        </feed>
        "#;
        let papers = parse_arxiv_xml(xml);
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].title, "Test Paper Title");
        assert_eq!(papers[0].abstract_text, "This is the abstract text.");
        assert!(papers[0].authors.contains("Alice Smith"));
        assert_eq!(papers[0].source, "arXiv");
    }

    #[test]
    fn test_parse_arxiv_xml_empty() {
        let xml = "<feed></feed>";
        let papers = parse_arxiv_xml(xml);
        assert_eq!(papers.len(), 0);
    }

    #[test]
    fn test_parse_arxiv_xml_whitespace_normalization() {
        let xml = r#"
        <feed>
          <entry>
            <id>http://arxiv.org/abs/2401.00002v1</id>
            <title>Title   With   Extra   Spaces</title>
            <summary>Abstract  with  spaces.</summary>
            <published>2024-01-15T00:00:00Z</published>
            <author><name>Test Author</name></author>
          </entry>
        </feed>
        "#;
        let papers = parse_arxiv_xml(xml);
        assert_eq!(papers[0].title, "Title With Extra Spaces");
    }
}
