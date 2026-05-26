use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::secrets;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub recipient: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicRequest {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub enabled: bool,
    pub frequency: String,      // "daily" or "weekly"
    pub days: Vec<String>,      // e.g. ["MON", "WED", "FRI"] (for weekly)
    pub time: String,           // "08:00" (HH:MM 24h format)
    pub topics: Vec<TopicRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionProfile {
    pub id: String,
    pub name: String,
    pub research_description: String,
    pub key_terms: Vec<String>,
    pub own_papers: Vec<String>,
    pub enabled: bool,
    #[serde(default)]
    pub last_scanned: Option<String>,
    #[serde(default)]
    pub last_overlap_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSettings {
    pub max_papers_per_source: u32,
    pub scan_days_back: u32,
    pub competition_threshold: u32,  // 0-100 score, papers above this are flagged
    pub auto_scan_with_newsletter: bool,
}

impl Default for ConflictSettings {
    fn default() -> Self {
        Self {
            max_papers_per_source: 200,
            scan_days_back: 30,
            competition_threshold: 30,
            auto_scan_with_newsletter: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLlmConfig {
    pub host: String,
    pub model: String,
    pub num_ctx: u32,
    pub num_predict_curation: i32,
    pub num_predict_scan: i32,
    pub temperature_curation: f32,
    pub temperature_scan: f32,
}

impl Default for LocalLlmConfig {
    fn default() -> Self {
        Self {
            host: "http://127.0.0.1:11434".to_string(),
            model: "qwen3:4b".to_string(),
            // 16k context fits the curation prompt comfortably and halves
            // KV-cache memory vs 32k. Conflict scan hits ~14k tokens of
            // input on 40 papers; we'd auto-truncate for higher counts.
            num_ctx: 16384,
            num_predict_curation: 4096,
            num_predict_scan: 8192,
            temperature_curation: 0.7,
            temperature_scan: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub gemini_api_key: String,
    #[serde(default)]
    pub claude_api_key: String,
    #[serde(default = "default_ai_provider")]
    pub ai_provider: String,
    pub output_dir: String,
    pub default_sources: Vec<String>,
    pub default_max_papers: u32,
    pub default_days_back: u32,
    pub email: EmailConfig,
    pub schedule: ScheduleConfig,
    #[serde(default)]
    pub conflict_profiles: Vec<CompetitionProfile>,
    #[serde(default)]
    pub conflict_settings: ConflictSettings,
    #[serde(default)]
    pub local_llm: LocalLlmConfig,
}

fn default_ai_provider() -> String {
    "gemini".to_string()
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_host: String::new(),
            smtp_port: 587,
            smtp_user: String::new(),
            smtp_password: String::new(),
            recipient: String::new(),
        }
    }
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            frequency: "weekly".to_string(),
            days: vec!["MON".to_string()],
            time: "08:00".to_string(),
            topics: Vec::new(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let output_dir = dirs_output_dir();
        Self {
            gemini_api_key: String::new(),
            claude_api_key: String::new(),
            ai_provider: default_ai_provider(),
            output_dir,
            default_sources: vec!["openalex".to_string(), "arxiv".to_string()],
            default_max_papers: 50,
            default_days_back: 90,
            email: EmailConfig::default(),
            schedule: ScheduleConfig::default(),
            conflict_profiles: Vec::new(),
            conflict_settings: ConflictSettings::default(),
            local_llm: LocalLlmConfig::default(),
        }
    }
}

fn dirs_output_dir() -> String {
    // Try to resolve Documents directory
    if let Some(docs) = dirs_next() {
        let newsletters = docs.join("newsletters");
        return newsletters.to_string_lossy().to_string();
    }
    // Fallback to home dir
    if let Ok(home) = std::env::var("USERPROFILE") {
        return format!("{home}\\Documents\\newsletters");
    }
    "newsletters".to_string()
}

fn dirs_next() -> Option<PathBuf> {
    // On Windows, USERPROFILE\Documents
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let docs = PathBuf::from(&profile).join("Documents");
        if docs.exists() {
            return Some(docs);
        }
    }
    None
}

/// Returns the path to the config file: `{app_config_dir}/config.json`
pub fn config_path(app: &tauri::AppHandle) -> PathBuf {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    config_dir.join("config.json")
}

/// Load config from disk, or return defaults if the file doesn't exist.
pub fn load_config(app: &tauri::AppHandle) -> AppConfig {
    let path = config_path(app);
    load_config_from_path(&path)
}

/// Load config from a specific file path (used by headless mode).
///
/// Secrets (SMTP password, Gemini API key) are pulled from the platform's
/// secret store, not the JSON file. If legacy plaintext secrets are found
/// inside the JSON, they are migrated to the secret store and stripped from
/// the file on the spot.
pub fn load_config_from_path(path: &Path) -> AppConfig {
    let mut cfg = if path.exists() {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    } else {
        AppConfig::default()
    };
    merge_secrets(&mut cfg, path);
    cfg
}

/// Move secrets between the on-disk JSON and the platform secret store.
///
/// * If the JSON has plaintext values (legacy or just-migrated config),
///   write them into the secret store and rewrite the file with those
///   fields cleared.
/// * Otherwise, populate the in-memory config from the secret store so
///   downstream code (email send, AI calls) sees the live secret.
fn merge_secrets(cfg: &mut AppConfig, path: &Path) {
    let mut migrated = false;

    // SMTP password
    if !cfg.email.smtp_password.is_empty() {
        if secrets::set(secrets::KEY_SMTP_PASSWORD, &cfg.email.smtp_password).is_ok() {
            migrated = true;
        }
    } else if let Some(stored) = secrets::get_or_none(secrets::KEY_SMTP_PASSWORD) {
        cfg.email.smtp_password = stored;
    }

    // Gemini API key
    if !cfg.gemini_api_key.is_empty() {
        if secrets::set(secrets::KEY_GEMINI_API_KEY, &cfg.gemini_api_key).is_ok() {
            migrated = true;
        }
    } else if let Some(stored) = secrets::get_or_none(secrets::KEY_GEMINI_API_KEY) {
        cfg.gemini_api_key = stored;
    }

    // Claude API key
    if !cfg.claude_api_key.is_empty() {
        if secrets::set(secrets::KEY_CLAUDE_API_KEY, &cfg.claude_api_key).is_ok() {
            migrated = true;
        }
    } else if let Some(stored) = secrets::get_or_none(secrets::KEY_CLAUDE_API_KEY) {
        cfg.claude_api_key = stored;
    }

    if migrated {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut sanitized = cfg.clone();
        sanitized.email.smtp_password = String::new();
        sanitized.gemini_api_key = String::new();
        sanitized.claude_api_key = String::new();
        if let Ok(text) = serde_json::to_string_pretty(&sanitized) {
            let _ = fs::write(path, text);
        }
    }
}

/// Persist config to disk, creating parent directories as needed.
///
/// Secrets are routed to the platform secret store. The JSON written to
/// disk has those fields blanked, so a stolen `config.json` has no
/// passwords or API keys in it.
pub fn save_config_to_disk(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // An empty value deletes the entry, so clearing a password in the UI
    // propagates correctly.
    secrets::set(secrets::KEY_SMTP_PASSWORD, &config.email.smtp_password)?;
    secrets::set(secrets::KEY_GEMINI_API_KEY, &config.gemini_api_key)?;
    secrets::set(secrets::KEY_CLAUDE_API_KEY, &config.claude_api_key)?;

    let mut sanitized = config.clone();
    sanitized.email.smtp_password = String::new();
    sanitized.gemini_api_key = String::new();
    sanitized.claude_api_key = String::new();

    let text = serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())?;

    ensure_output_dir(&config.output_dir);
    Ok(())
}

/// Ensure the newsletter output directory exists.
pub fn ensure_output_dir(dir: &str) {
    if !dir.is_empty() {
        let _ = fs::create_dir_all(Path::new(dir));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_expected_sources() {
        let cfg = AppConfig::default();
        assert!(cfg.default_sources.contains(&"openalex".to_string()));
        assert!(cfg.default_sources.contains(&"arxiv".to_string()));
        assert_eq!(cfg.default_max_papers, 50);
        assert_eq!(cfg.default_days_back, 90);
        assert!(!cfg.email.enabled);
        assert!(!cfg.schedule.enabled);
    }

    #[test]
    fn test_config_round_trip_serialization() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.gemini_api_key, restored.gemini_api_key);
        assert_eq!(cfg.default_max_papers, restored.default_max_papers);
        assert_eq!(cfg.default_sources, restored.default_sources);
    }

    #[test]
    fn test_email_config_default_port() {
        let email_cfg = EmailConfig::default();
        assert_eq!(email_cfg.smtp_port, 587);
    }

    #[test]
    fn test_schedule_config_defaults() {
        let sched = ScheduleConfig::default();
        assert_eq!(sched.frequency, "weekly");
        assert_eq!(sched.days, vec!["MON".to_string()]);
        assert_eq!(sched.time, "08:00");
        assert!(!sched.enabled);
    }

    #[test]
    fn test_conflict_settings_defaults() {
        let settings = ConflictSettings::default();
        assert_eq!(settings.max_papers_per_source, 200);
        assert_eq!(settings.scan_days_back, 30);
        assert_eq!(settings.competition_threshold, 30);
        assert!(!settings.auto_scan_with_newsletter);
    }

    #[test]
    fn test_local_llm_defaults() {
        let cfg = LocalLlmConfig::default();
        assert_eq!(cfg.host, "http://127.0.0.1:11434");
        assert_eq!(cfg.model, "qwen3:4b");
        assert_eq!(cfg.num_ctx, 16384);
        assert!(cfg.temperature_curation > cfg.temperature_scan,
                "curation should be more creative than the JSON scan");
    }

    #[test]
    fn test_app_config_includes_local_llm() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.local_llm.model, "qwen3:4b");
    }

    #[test]
    fn test_legacy_config_without_local_llm_deserializes() {
        // Older configs from before this feature didn't include the
        // local_llm field. Make sure they still load (serde default).
        let legacy = r#"{
            "gemini_api_key": "",
            "ai_provider": "gemini",
            "output_dir": "x",
            "default_sources": [],
            "default_max_papers": 10,
            "default_days_back": 30,
            "email": {
                "enabled": false,
                "smtp_host": "",
                "smtp_port": 587,
                "smtp_user": "",
                "smtp_password": "",
                "recipient": ""
            },
            "schedule": {
                "enabled": false,
                "frequency": "weekly",
                "days": ["MON"],
                "time": "08:00",
                "topics": []
            }
        }"#;
        let cfg: AppConfig = serde_json::from_str(legacy).expect("legacy config should parse");
        assert_eq!(cfg.local_llm.model, "qwen3:4b");
    }

    #[test]
    fn test_default_config_has_empty_conflict_profiles() {
        let cfg = AppConfig::default();
        assert!(cfg.conflict_profiles.is_empty());
        assert_eq!(cfg.conflict_settings.max_papers_per_source, 200);
        assert_eq!(cfg.conflict_settings.scan_days_back, 30);
        assert_eq!(cfg.conflict_settings.competition_threshold, 30);
        assert!(!cfg.conflict_settings.auto_scan_with_newsletter);
    }
}
