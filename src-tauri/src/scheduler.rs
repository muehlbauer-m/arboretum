//! Cross-platform scheduler.
//!
//! * Windows: `schtasks.exe` creates a scheduled task in Windows Task Scheduler.
//! * macOS: writes a launchd agent plist to `~/Library/LaunchAgents/` and loads
//!   it with `launchctl`.
//!
//! Public API (`create_scheduled_task`, `delete_scheduled_task`, `get_task_info`)
//! has the same signature on every platform; the implementation is cfg-gated.

use serde::{Deserialize, Serialize};

pub const TASK_NAME: &str = "ResearchNewsletter";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub exists: bool,
    pub next_run: Option<String>,
    pub status: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Windows implementation (schtasks.exe)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(windows)]
pub fn create_scheduled_task(
    app_exe_path: &str,
    frequency: &str,
    days: &[String],
    time: &str,
) -> Result<(), String> {
    use std::process::Command;

    let _ = delete_scheduled_task();

    let mut args = vec![
        "/Create".to_string(),
        "/TN".to_string(),
        TASK_NAME.to_string(),
        "/TR".to_string(),
        format!("\"{}\" --scheduled-run", app_exe_path),
        "/ST".to_string(),
        time.to_string(),
        "/F".to_string(),
    ];

    match frequency {
        "daily" => {
            args.extend(["/SC".to_string(), "DAILY".to_string()]);
        }
        "weekly" => {
            if days.is_empty() {
                return Err("Weekly schedule requires at least one day".to_string());
            }
            args.extend([
                "/SC".to_string(),
                "WEEKLY".to_string(),
                "/D".to_string(),
                days.join(","),
            ]);
        }
        _ => return Err(format!("Unknown frequency: {frequency}")),
    }

    let output = Command::new("schtasks")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run schtasks: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!(
            "schtasks /Create failed: {}{}",
            stderr.trim(),
            if stdout.trim().is_empty() {
                String::new()
            } else {
                format!(" | {}", stdout.trim())
            }
        ))
    }
}

#[cfg(windows)]
pub fn delete_scheduled_task() -> Result<(), String> {
    use std::process::Command;
    let output = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .output()
        .map_err(|e| format!("Failed to run schtasks: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("schtasks /Delete failed: {}", stderr.trim()))
    }
}

#[cfg(windows)]
pub fn get_task_info() -> TaskInfo {
    use std::process::Command;
    let output = match Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME, "/FO", "CSV", "/V"])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
            return TaskInfo {
                exists: false,
                next_run: None,
                status: None,
            }
        }
    };

    if !output.status.success() {
        return TaskInfo {
            exists: false,
            next_run: None,
            status: None,
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_task_csv(&stdout)
}

#[cfg(windows)]
fn parse_task_csv(csv_output: &str) -> TaskInfo {
    let lines: Vec<&str> = csv_output.lines().collect();
    if lines.len() < 2 {
        return TaskInfo {
            exists: false,
            next_run: None,
            status: None,
        };
    }

    let headers = parse_csv_line(lines[0]);
    let values = parse_csv_line(lines[1]);

    let next_run_idx = headers.iter().position(|h| h.contains("Next Run Time"));
    let status_idx = headers.iter().position(|h| h == "Status" || h == "\"Status\"");

    let next_run = next_run_idx
        .and_then(|i| values.get(i))
        .map(|v| v.trim_matches('"').to_string())
        .filter(|v| !v.is_empty() && v != "N/A");

    let status = status_idx
        .and_then(|i| values.get(i))
        .map(|v| v.trim_matches('"').to_string())
        .filter(|v| !v.is_empty());

    TaskInfo {
        exists: true,
        next_run,
        status,
    }
}

#[cfg(windows)]
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    fields.push(current.trim().to_string());
    fields
}

// ═══════════════════════════════════════════════════════════════════════════
//  macOS implementation (launchd via launchctl)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "macos")]
const LAUNCHD_LABEL: &str = "com.research.newsletter.scheduled";

#[cfg(target_os = "macos")]
fn plist_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("$HOME is not set")?;
    Ok(std::path::PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LAUNCHD_LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn day_to_launchd_weekday(day: &str) -> Option<u8> {
    // launchd Weekday: SUN=0 MON=1 TUE=2 WED=3 THU=4 FRI=5 SAT=6
    Some(match day.to_ascii_uppercase().as_str() {
        "SUN" => 0,
        "MON" => 1,
        "TUE" => 2,
        "WED" => 3,
        "THU" => 4,
        "FRI" => 5,
        "SAT" => 6,
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
fn build_launchd_plist(
    exe_path: &str,
    frequency: &str,
    days: &[String],
    time: &str,
) -> Result<String, String> {
    let (hour, minute): (u8, u8) = {
        let mut parts = time.split(':');
        let h = parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("Invalid time: {time}"))?;
        let m = parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("Invalid time: {time}"))?;
        (h, m)
    };

    let calendar = match frequency {
        "daily" => format!(
            concat!(
                "    <key>StartCalendarInterval</key>\n",
                "    <dict>\n",
                "      <key>Hour</key><integer>{hour}</integer>\n",
                "      <key>Minute</key><integer>{minute}</integer>\n",
                "    </dict>"
            ),
            hour = hour,
            minute = minute,
        ),
        "weekly" => {
            if days.is_empty() {
                return Err("Weekly schedule requires at least one day".into());
            }
            let mut entries = String::new();
            for day in days {
                let weekday = day_to_launchd_weekday(day)
                    .ok_or_else(|| format!("Unknown day: {day}"))?;
                entries.push_str(&format!(
                    concat!(
                        "      <dict>\n",
                        "        <key>Weekday</key><integer>{weekday}</integer>\n",
                        "        <key>Hour</key><integer>{hour}</integer>\n",
                        "        <key>Minute</key><integer>{minute}</integer>\n",
                        "      </dict>\n"
                    ),
                    weekday = weekday,
                    hour = hour,
                    minute = minute,
                ));
            }
            format!(
                "    <key>StartCalendarInterval</key>\n    <array>\n{entries}    </array>"
            )
        }
        _ => return Err(format!("Unknown frequency: {frequency}")),
    };

    Ok(format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\">\n",
            "  <dict>\n",
            "    <key>Label</key>\n",
            "    <string>{label}</string>\n",
            "    <key>ProgramArguments</key>\n",
            "    <array>\n",
            "      <string>{exe}</string>\n",
            "      <string>--scheduled-run</string>\n",
            "    </array>\n",
            "{calendar}\n",
            "    <key>RunAtLoad</key>\n",
            "    <false/>\n",
            "    <key>StandardOutPath</key>\n",
            "    <string>/tmp/{label}.out.log</string>\n",
            "    <key>StandardErrorPath</key>\n",
            "    <string>/tmp/{label}.err.log</string>\n",
            "  </dict>\n",
            "</plist>\n"
        ),
        label = LAUNCHD_LABEL,
        exe = exe_path,
        calendar = calendar,
    ))
}

#[cfg(target_os = "macos")]
pub fn create_scheduled_task(
    app_exe_path: &str,
    frequency: &str,
    days: &[String],
    time: &str,
) -> Result<(), String> {
    use std::process::Command;

    let path = plist_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Creating LaunchAgents dir: {e}"))?;
    }

    // Unload any existing version first (ignore errors — it may not be loaded).
    if path.exists() {
        let _ = Command::new("launchctl")
            .args(["unload", path.to_string_lossy().as_ref()])
            .output();
    }

    let contents = build_launchd_plist(app_exe_path, frequency, days, time)?;
    std::fs::write(&path, contents).map_err(|e| format!("Writing plist: {e}"))?;

    let output = Command::new("launchctl")
        .args(["load", "-w", path.to_string_lossy().as_ref()])
        .output()
        .map_err(|e| format!("Running launchctl: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("launchctl load failed: {}", stderr.trim()))
    }
}

#[cfg(target_os = "macos")]
pub fn delete_scheduled_task() -> Result<(), String> {
    use std::process::Command;
    let path = plist_path()?;
    if !path.exists() {
        return Ok(());
    }
    let _ = Command::new("launchctl")
        .args(["unload", path.to_string_lossy().as_ref()])
        .output();
    std::fs::remove_file(&path).map_err(|e| format!("Removing plist: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn get_task_info() -> TaskInfo {
    use std::process::Command;

    let path = match plist_path() {
        Ok(p) => p,
        Err(_) => {
            return TaskInfo {
                exists: false,
                next_run: None,
                status: None,
            }
        }
    };

    if !path.exists() {
        return TaskInfo {
            exists: false,
            next_run: None,
            status: None,
        };
    }

    // `launchctl list <label>` exits 0 iff the agent is loaded.
    let loaded = Command::new("launchctl")
        .args(["list", LAUNCHD_LABEL])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    TaskInfo {
        exists: true,
        next_run: None,
        status: Some(if loaded { "Loaded" } else { "Unloaded" }.to_string()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  Fallback for unsupported platforms (Linux, etc.)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(not(any(windows, target_os = "macos")))]
pub fn create_scheduled_task(
    _app_exe_path: &str,
    _frequency: &str,
    _days: &[String],
    _time: &str,
) -> Result<(), String> {
    Err("Scheduling is only supported on Windows and macOS.".into())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn delete_scheduled_task() -> Result<(), String> {
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn get_task_info() -> TaskInfo {
    TaskInfo {
        exists: false,
        next_run: None,
        status: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn test_parse_csv_line_simple() {
        let line = r#""HostName","TaskName","Next Run Time","Status""#;
        let fields = parse_csv_line(line);
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0], "HostName");
        assert_eq!(fields[1], "TaskName");
        assert_eq!(fields[2], "Next Run Time");
        assert_eq!(fields[3], "Status");
    }

    #[test]
    fn test_parse_task_csv_valid() {
        let csv = r#""HostName","TaskName","Next Run Time","Status","Logon Mode"
"DESKTOP","ResearchNewsletter","3/26/2026 8:00:00 AM","Ready","Interactive only""#;
        let info = parse_task_csv(csv);
        assert!(info.exists);
        assert_eq!(info.next_run, Some("3/26/2026 8:00:00 AM".to_string()));
        assert_eq!(info.status, Some("Ready".to_string()));
    }

    #[test]
    fn test_parse_task_csv_empty() {
        let info = parse_task_csv("");
        assert!(!info.exists);
        assert!(info.next_run.is_none());
        assert!(info.status.is_none());
    }

    #[test]
    fn test_parse_task_csv_single_line() {
        let csv = r#""HostName","TaskName","Next Run Time","Status""#;
        let info = parse_task_csv(csv);
        assert!(!info.exists);
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;

    #[test]
    fn build_plist_daily() {
        let plist = build_launchd_plist("/Applications/App.app/Contents/MacOS/App", "daily", &[], "08:30").unwrap();
        assert!(plist.contains("<string>com.research.newsletter.scheduled</string>"));
        assert!(plist.contains("<key>Hour</key><integer>8</integer>"));
        assert!(plist.contains("<key>Minute</key><integer>30</integer>"));
        assert!(plist.contains("--scheduled-run"));
        assert!(plist.contains("StartCalendarInterval"));
    }

    #[test]
    fn build_plist_weekly_multi_day() {
        let days = vec!["MON".to_string(), "WED".to_string(), "FRI".to_string()];
        let plist = build_launchd_plist("/usr/local/bin/app", "weekly", &days, "07:00").unwrap();
        // Three weekday dicts in the array
        assert_eq!(plist.matches("<key>Weekday</key>").count(), 3);
        assert!(plist.contains("<integer>1</integer>")); // MON
        assert!(plist.contains("<integer>3</integer>")); // WED
        assert!(plist.contains("<integer>5</integer>")); // FRI
    }

    #[test]
    fn build_plist_rejects_unknown_frequency() {
        let err = build_launchd_plist("/x", "hourly", &[], "08:00").unwrap_err();
        assert!(err.contains("hourly"));
    }

    #[test]
    fn build_plist_rejects_weekly_no_days() {
        let err = build_launchd_plist("/x", "weekly", &[], "08:00").unwrap_err();
        assert!(err.to_lowercase().contains("at least one day"));
    }

    #[test]
    fn day_mapping() {
        assert_eq!(day_to_launchd_weekday("MON"), Some(1));
        assert_eq!(day_to_launchd_weekday("sun"), Some(0));
        assert_eq!(day_to_launchd_weekday("xyz"), None);
    }
}
