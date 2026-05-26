pub mod claude_api;
pub mod config;
pub mod conflict;
pub mod email;
pub mod gemini;
pub mod hardware;
pub mod local_llm;
pub mod pipeline;
pub mod scheduler;
pub mod secrets;
pub mod sources;

use config::{load_config, save_config_to_disk, AppConfig};
use pipeline::{GenerateResult, NewsletterMeta, TopicRequest};
use reqwest::Client;
use std::sync::Arc;
use tauri::State;
use tokio::sync::{oneshot, Mutex};

// ─── Shared State ─────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct PullState {
    pub current_model: Option<String>,
    pub cancel_tx: Option<oneshot::Sender<()>>,
}

pub struct AppState {
    pub http_client: Client,
    pub pull_state: Arc<Mutex<PullState>>,
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn get_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    Ok(load_config(&app))
}

#[tauri::command]
async fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    save_config_to_disk(&app, &config)?;
    Ok(())
}

#[tauri::command]
async fn generate_newsletter(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    topics: Vec<TopicRequest>,
    sources: Vec<String>,
    max_papers: u32,
    days_back: u32,
) -> Result<Vec<GenerateResult>, String> {
    let config = load_config(&app);

    match config.ai_provider.as_str() {
        "claude" => {
            if config.claude_api_key.is_empty() {
                return Err(
                    "Claude API key is not set. Please configure it in Settings.".to_string()
                );
            }
        }
        "local" => {
            let client = state.http_client.clone();
            local_llm::check_status(&client, &config.local_llm.host)
                .await
                .map_err(|e| {
                    format!(
                        "Local model unavailable. Is Ollama running? Error: {e}"
                    )
                })?;
        }
        _ => {
            if config.gemini_api_key.is_empty() {
                return Err(
                    "Gemini API key is not set. Please configure it in Settings.".to_string()
                );
            }
        }
    }

    // Ensure output directory exists
    config::ensure_output_dir(&config.output_dir);

    let client = state.http_client.clone();

    // Run all topics concurrently
    let mut handles = Vec::new();
    for topic in topics {
        let topic_query = topic.query.clone();
        let app_clone = app.clone();
        let client_clone = client.clone();
        let sources_clone = sources.clone();
        let config_clone = config.clone();

        let handle = tokio::spawn(async move {
            pipeline::run_single_topic(
                app_clone,
                client_clone,
                topic,
                sources_clone,
                max_papers,
                days_back,
                config_clone,
            )
            .await
        });
        handles.push((topic_query, handle));
    }

    let mut results = Vec::new();
    for (topic_query, handle) in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => results.push(GenerateResult {
                topic: topic_query,
                title: String::new(),
                path: String::new(),
                error: Some(format!("Task panicked: {e}")),
            }),
        }
    }

    Ok(results)
}

#[tauri::command]
async fn list_newsletters(output_dir: String) -> Result<Vec<NewsletterMeta>, String> {
    Ok(pipeline::list_newsletters(&output_dir))
}

#[tauri::command]
async fn read_newsletter(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Cannot read file '{path}': {e}"))
}

#[tauri::command]
async fn send_newsletter_email(
    app: tauri::AppHandle,
    path: String,
) -> Result<(), String> {
    let config = load_config(&app);
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Cannot read file: {e}"))?;

    // Derive subject from first heading
    let subject = content
        .lines()
        .find(|l| l.starts_with('#'))
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .unwrap_or_else(|| "Arboretum digest".to_string());

    let email_cfg = config.email.clone();
    tokio::task::spawn_blocking(move || email::send_email(&content, &subject, &email_cfg))
        .await
        .map_err(|e| format!("Email task failed: {e}"))
        .and_then(|r| r)
}

#[tauri::command]
async fn test_email_connection(
    email_config: config::EmailConfig,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || email::test_connection(&email_config))
        .await
        .map_err(|e| format!("Connection test task failed: {e}"))
        .and_then(|r| r)
}

#[tauri::command]
async fn scan_conflicts(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    profiles: Vec<config::CompetitionProfile>,
    sources: Vec<String>,
    max_papers: u32,
    days_back: u32,
) -> Result<Vec<conflict::ConflictScanResult>, String> {
    let cfg = load_config(&app);
    let client = state.http_client.clone();

    let mut handles = Vec::new();
    for profile in profiles {
        let profile_name = profile.name.clone();
        let app_clone = app.clone();
        let client_clone = client.clone();
        let sources_clone = sources.clone();
        let config_clone = cfg.clone();

        let handle = tokio::spawn(async move {
            conflict::run_conflict_scan(
                app_clone,
                client_clone,
                profile,
                sources_clone,
                max_papers,
                days_back,
                config_clone,
            )
            .await
        });
        handles.push((profile_name, handle));
    }

    let mut results = Vec::new();
    for (profile_name, handle) in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => results.push(conflict::ConflictScanResult {
                profile_id: String::new(),
                profile_name,
                overlaps: Vec::new(),
                scanned_at: chrono::Utc::now().to_rfc3339(),
                papers_checked: 0,
                error: Some(format!("Task panicked: {e}")),
            }),
        }
    }

    Ok(results)
}

// ─── Local AI Commands ───────────────────────────────────────────────────────

#[tauri::command]
async fn detect_hardware() -> Result<hardware::HardwareProfile, String> {
    Ok(hardware::detect_profile())
}

#[tauri::command]
async fn recommend_local_model() -> Result<hardware::ModelRecommendation, String> {
    let profile = hardware::detect_profile();
    Ok(hardware::recommend(&profile))
}

#[tauri::command]
async fn list_known_local_models() -> Result<Vec<hardware::ModelOption>, String> {
    let profile = hardware::detect_profile();
    Ok(hardware::known_models(&profile))
}

#[derive(serde::Serialize)]
struct OllamaStatus {
    running: bool,
    version: Option<String>,
    error: Option<String>,
}

#[tauri::command]
async fn check_ollama_status(
    state: State<'_, AppState>,
    host: String,
) -> Result<OllamaStatus, String> {
    let client = state.http_client.clone();
    match local_llm::check_status(&client, &host).await {
        Ok(version) => Ok(OllamaStatus {
            running: true,
            version: Some(version),
            error: None,
        }),
        Err(e) => Ok(OllamaStatus {
            running: false,
            version: None,
            error: Some(e),
        }),
    }
}

#[tauri::command]
async fn list_installed_local_models(
    state: State<'_, AppState>,
    host: String,
) -> Result<Vec<local_llm::LocalModelInfo>, String> {
    let client = state.http_client.clone();
    local_llm::list_models(&client, &host).await
}

#[derive(serde::Serialize)]
struct ActivePull {
    model: Option<String>,
}

#[tauri::command]
async fn pull_local_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    host: String,
    model: String,
) -> Result<(), String> {
    let client = state.http_client.clone();
    let pull_state = state.pull_state.clone();

    // Replace any prior cancel-tx with a fresh one. Dropping the previous
    // sender (if any) signals a still-running pull to wind down.
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    {
        let mut guard = pull_state.lock().await;
        // Sending on the old tx (if a previous pull is racing us) tells it
        // to stop. If it already finished, send fails silently.
        if let Some(prev) = guard.cancel_tx.take() {
            let _ = prev.send(());
        }
        guard.current_model = Some(model.clone());
        guard.cancel_tx = Some(cancel_tx);
    }

    let result = local_llm::pull_model(&app, &client, &host, &model, Some(cancel_rx)).await;

    // Clear pull state now that we're done (success, error, or cancelled).
    {
        let mut guard = pull_state.lock().await;
        if guard.current_model.as_deref() == Some(model.as_str()) {
            guard.current_model = None;
            guard.cancel_tx = None;
        }
    }

    result
}

#[tauri::command]
async fn cancel_local_pull(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.pull_state.lock().await;
    if let Some(tx) = guard.cancel_tx.take() {
        let _ = tx.send(());
    }
    guard.current_model = None;
    Ok(())
}

#[tauri::command]
async fn get_active_pull(state: State<'_, AppState>) -> Result<ActivePull, String> {
    let guard = state.pull_state.lock().await;
    Ok(ActivePull {
        model: guard.current_model.clone(),
    })
}

// ─── Schedule Commands (Windows Task Scheduler) ──────────────────────────────

#[tauri::command]
async fn create_schedule(config: AppConfig) -> Result<(), String> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Cannot determine exe path: {e}"))?
        .to_string_lossy()
        .to_string();

    scheduler::create_scheduled_task(
        &exe_path,
        &config.schedule.frequency,
        &config.schedule.days,
        &config.schedule.time,
    )
}

#[tauri::command]
async fn delete_schedule() -> Result<(), String> {
    scheduler::delete_scheduled_task()
}

#[tauri::command]
async fn get_schedule_status() -> Result<scheduler::TaskInfo, String> {
    Ok(scheduler::get_task_info())
}

// ─── App Entry Point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_client = Client::builder()
        .use_rustls_tls()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("Arboretum/0.1")
        .build()
        .expect("Failed to build HTTP client");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            http_client,
            pull_state: Arc::new(Mutex::new(PullState::default())),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            generate_newsletter,
            list_newsletters,
            read_newsletter,
            send_newsletter_email,
            test_email_connection,
            scan_conflicts,
            create_schedule,
            delete_schedule,
            get_schedule_status,
            detect_hardware,
            recommend_local_model,
            list_known_local_models,
            check_ollama_status,
            list_installed_local_models,
            pull_local_model,
            cancel_local_pull,
            get_active_pull,
        ])
        .setup(|app| {
            // Load config and ensure output directory exists
            let config = load_config(app.handle());
            config::ensure_output_dir(&config.output_dir);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
