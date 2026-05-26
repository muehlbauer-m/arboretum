// Prevents an extra console window on Windows release builds.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() {
    // Check for --scheduled-run flag for headless mode
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--scheduled-run") {
        run_scheduled();
    } else {
        research_newsletter_lib::run();
    }
}

/// Headless mode: run the pipeline for each scheduled topic, send emails, notify, and exit.
///
/// This is invoked by Windows Task Scheduler. No window is opened.
fn run_scheduled() {
    use research_newsletter_lib::config;
    use research_newsletter_lib::email;
    use research_newsletter_lib::gemini::{extract_keywords, summarize_with_gemini, Paper};
    use research_newsletter_lib::claude_api;
    use research_newsletter_lib::local_llm;
    use research_newsletter_lib::pipeline::save_newsletter;
    use research_newsletter_lib::sources::openalex::search_openalex;
    use research_newsletter_lib::sources::arxiv::search_arxiv;

    // Resolve the platform's config path (see `resolve_config_path`).
    let config_path = resolve_config_path();
    let cfg = config::load_config_from_path(&config_path);

    if !cfg.schedule.enabled || cfg.schedule.topics.is_empty() {
        eprintln!("[scheduled-run] Schedule is disabled or no topics configured. Exiting.");
        return;
    }

    // Ensure output directory exists
    config::ensure_output_dir(&cfg.output_dir);

    // Create a tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        let client = match reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(120))
            .user_agent("Arboretum/0.1")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[scheduled-run] Failed to build HTTP client: {e}");
                show_notification("Arboretum", &format!("Error: failed to build HTTP client: {e}"));
                return;
            }
        };

        let sources = &cfg.default_sources;
        let max_papers = cfg.default_max_papers;
        let days_back = cfg.default_days_back;
        let provider = cfg.ai_provider.as_str();
        let api_key = &cfg.gemini_api_key;
        let claude_key = &cfg.claude_api_key;

        let mut success_count = 0u32;
        let mut error_count = 0u32;
        let mut generated_paths: Vec<String> = Vec::new();

        for topic in &cfg.schedule.topics {
            let query = &topic.query;
            if query.trim().is_empty() {
                continue;
            }
            eprintln!("[scheduled-run] Processing topic: {query}");

            // Step 1: Extract keywords
            let kw_result = match match provider {
                "claude" => claude_api::extract_keywords(&client, query, claude_key).await,
                "local"  => local_llm::extract_keywords(&client, &cfg.local_llm, query).await,
                _        => extract_keywords(&client, query, api_key).await,
            } {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[scheduled-run]   Keyword extraction failed: {e}");
                    error_count += 1;
                    continue;
                }
            };
            eprintln!("[scheduled-run]   Keywords: \"{}\"  Title: \"{}\"", kw_result.keywords, kw_result.title);

            // Step 2: Fetch papers
            let mut all_papers: Vec<Paper> = Vec::new();
            let per_source = if sources.is_empty() {
                max_papers
            } else {
                (max_papers / sources.len() as u32).max(10)
            };

            if sources.contains(&"openalex".to_string()) {
                match search_openalex(&client, &kw_result.keywords, per_source, days_back).await {
                    Ok(papers) => {
                        eprintln!("[scheduled-run]   OpenAlex: {} papers", papers.len());
                        all_papers.extend(papers);
                    }
                    Err(e) => {
                        eprintln!("[scheduled-run]   OpenAlex error: {e}");
                    }
                }
            }

            if sources.contains(&"arxiv".to_string()) {
                match search_arxiv(&client, &kw_result.keywords, per_source).await {
                    Ok(papers) => {
                        eprintln!("[scheduled-run]   arXiv: {} papers", papers.len());
                        all_papers.extend(papers);
                    }
                    Err(e) => {
                        eprintln!("[scheduled-run]   arXiv error: {e}");
                    }
                }
            }

            if all_papers.is_empty() {
                eprintln!("[scheduled-run]   No papers found for \"{query}\". Skipping.");
                error_count += 1;
                continue;
            }

            // Step 3: Summarize
            let summary = match match provider {
                "claude" => claude_api::summarize(&client, &all_papers, query, claude_key).await,
                "local"  => local_llm::summarize(None, &client, &cfg.local_llm, &all_papers, query).await,
                _        => summarize_with_gemini(&client, &all_papers, query, api_key).await,
            } {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[scheduled-run]   Summarization failed: {e}");
                    error_count += 1;
                    continue;
                }
            };

            // Step 4: Save
            match save_newsletter(&summary, &kw_result.title, &cfg.output_dir, provider) {
                Ok(path) => {
                    eprintln!("[scheduled-run]   Saved: {path}");
                    generated_paths.push(path);
                    success_count += 1;
                }
                Err(e) => {
                    eprintln!("[scheduled-run]   Save failed: {e}");
                    error_count += 1;
                }
            }
        }

        // Send emails for each generated newsletter if email is enabled
        if cfg.email.enabled {
            for path in &generated_paths {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let subject = content
                            .lines()
                            .find(|l| l.starts_with('#'))
                            .map(|l| l.trim_start_matches('#').trim().to_string())
                            .unwrap_or_else(|| "Arboretum digest".to_string());

                        match email::send_email(&content, &subject, &cfg.email) {
                            Ok(()) => eprintln!("[scheduled-run]   Email sent for: {path}"),
                            Err(e) => eprintln!("[scheduled-run]   Email failed for {path}: {e}"),
                        }
                    }
                    Err(e) => {
                        eprintln!("[scheduled-run]   Cannot read {path} for email: {e}");
                    }
                }
            }
        }

        // Show a native desktop notification with the summary.
        let body = if error_count == 0 {
            format!("Generated {} newsletter(s) successfully.", success_count)
        } else {
            format!(
                "Generated {} newsletter(s), {} failed.",
                success_count, error_count
            )
        };
        show_notification("Arboretum", &body);

        eprintln!("[scheduled-run] Done. {} success, {} errors.", success_count, error_count);
    });
}

/// Resolve the config file path for headless mode.
///
/// Tauri v2 stores plugin state at the platform's standard config dir:
/// * Windows: `%APPDATA%\{identifier}\config.json`
/// * macOS: `~/Library/Application Support/{identifier}/config.json`
/// * Linux: `$XDG_CONFIG_HOME/{identifier}/config.json` (fallback
///   `~/.config/{identifier}/config.json`)
fn resolve_config_path() -> PathBuf {
    const IDENTIFIER: &str = "com.research.newsletter";

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join(IDENTIFIER).join("config.json");
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile)
                .join("AppData")
                .join("Roaming")
                .join(IDENTIFIER)
                .join("config.json");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(IDENTIFIER)
                .join("config.json");
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join(IDENTIFIER).join("config.json");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join(IDENTIFIER)
                .join("config.json");
        }
    }

    // Last-resort fallback — relative to CWD.
    PathBuf::from("config.json")
}

/// Show a native desktop notification.
///
/// `notify-rust` dispatches to the right system API on each platform (toast on
/// Windows, NSUserNotification / NSUserNotificationCenter on macOS, libnotify
/// on Linux).
fn show_notification(summary: &str, body: &str) {
    #[cfg(not(test))]
    {
        if let Err(e) = notify_rust::Notification::new()
            .summary(summary)
            .body(body)
            .show()
        {
            eprintln!("[scheduled-run] Toast notification failed: {e}");
        }
    }
    #[cfg(test)]
    {
        let _ = (summary, body);
    }
}
