//! Hardware detection + model recommendation for the local-AI provider.
//!
//! We can't reliably probe GPU VRAM cross-platform without bringing in heavy
//! deps, so the recommendation logic uses RAM + CPU brand as a proxy. The
//! result is surfaced to the user as a *suggestion* with a `reason` string —
//! they can always override.

use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub os: String,
    pub total_ram_gb: f64,
    pub free_ram_gb: f64,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub is_apple_silicon: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub model: String,
    pub display_name: String,
    pub reason: String,
    pub tier: String, // "comfortable" | "tight" | "minimum" | "unsupported"
    pub estimated_disk_gb: f64,
    pub estimated_ram_gb: f64,
}

pub fn detect_profile() -> HardwareProfile {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    let total_ram_bytes = sys.total_memory();
    let free_ram_bytes = sys.available_memory();

    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let cpu_cores = sys.physical_core_count().unwrap_or_else(|| sys.cpus().len());

    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH;

    // Apple Silicon = macOS + ARM64. The combination of Metal + unified
    // memory is what makes the recommendation jump up a tier.
    let is_apple_silicon = os == "macos" && arch == "aarch64";

    HardwareProfile {
        os,
        total_ram_gb: bytes_to_gb(total_ram_bytes),
        free_ram_gb: bytes_to_gb(free_ram_bytes),
        cpu_brand,
        cpu_cores,
        is_apple_silicon,
    }
}

pub fn recommend(profile: &HardwareProfile) -> ModelRecommendation {
    classify(profile.total_ram_gb, profile.is_apple_silicon)
}

fn classify(total_ram_gb: f64, apple_silicon: bool) -> ModelRecommendation {
    // Tier rules — see the quality-test report for why these break
    // where they break:
    //   - Apple Silicon ≥16 GB unified → qwen3:8b runs at 15-25 tok/s
    //   - x86 with ≥32 GB → qwen3:8b is comfortable on CPU
    //   - 16-32 GB on either platform → qwen3:4b fits with headroom
    //   - <16 GB → only the smallest model is honest

    // OS-reported "total" is always slightly under nominal spec (firmware
    // reservation, integrated graphics aperture, etc.). Round up by ~0.5 GB
    // when comparing against tier thresholds so a "16 GB" laptop that
    // reports 15.92 GB lands in the right tier.
    let effective_ram_gb = total_ram_gb + 0.5;

    if apple_silicon && effective_ram_gb >= 16.0 {
        return ModelRecommendation {
            model: "qwen3:8b".to_string(),
            display_name: "Qwen 3 · 8B".to_string(),
            reason: format!(
                "Apple Silicon with {:.0} GB unified memory — 8B Q4 fits comfortably and uses Metal acceleration.",
                total_ram_gb
            ),
            tier: "comfortable".to_string(),
            estimated_disk_gb: 5.2,
            estimated_ram_gb: 6.5,
        };
    }

    if effective_ram_gb >= 32.0 {
        return ModelRecommendation {
            model: "qwen3:8b".to_string(),
            display_name: "Qwen 3 · 8B".to_string(),
            reason: format!(
                "{:.0} GB system RAM — 8B Q4 fits with room for the OS and a 32k context window.",
                total_ram_gb
            ),
            tier: "comfortable".to_string(),
            estimated_disk_gb: 5.2,
            estimated_ram_gb: 10.5,
        };
    }

    if effective_ram_gb >= 16.0 {
        return ModelRecommendation {
            model: "qwen3:4b".to_string(),
            display_name: "Qwen 3 · 4B".to_string(),
            reason: format!(
                "{:.0} GB system RAM — 4B Q4 leaves headroom for the OS; 8B would be tight.",
                total_ram_gb
            ),
            tier: "tight".to_string(),
            estimated_disk_gb: 2.5,
            estimated_ram_gb: 4.5,
        };
    }

    if effective_ram_gb >= 8.0 {
        return ModelRecommendation {
            model: "qwen3:1.7b".to_string(),
            display_name: "Qwen 3 · 1.7B".to_string(),
            reason: format!(
                "{:.0} GB system RAM is below comfortable for 4B+ models — a 1.7B model is the honest pick. Curation quality drops noticeably.",
                total_ram_gb
            ),
            tier: "minimum".to_string(),
            estimated_disk_gb: 1.4,
            estimated_ram_gb: 2.5,
        };
    }

    ModelRecommendation {
        model: "qwen3:1.7b".to_string(),
        display_name: "Qwen 3 · 1.7B".to_string(),
        reason: format!(
            "Only {:.1} GB RAM detected — local AI is unlikely to give acceptable results. Consider Claude or Gemini.",
            total_ram_gb
        ),
        tier: "unsupported".to_string(),
        estimated_disk_gb: 1.4,
        estimated_ram_gb: 2.5,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOption {
    pub model: String,
    pub display_name: String,
    pub size_label: String,
    pub fits: bool,
    pub note: String,
}

/// All models we know about, annotated with whether the current hardware
/// can run them. Used to populate the model picker.
pub fn known_models(profile: &HardwareProfile) -> Vec<ModelOption> {
    let r = profile.total_ram_gb;
    vec![
        ModelOption {
            model: "qwen3:1.7b".to_string(),
            display_name: "Qwen 3 · 1.7B".to_string(),
            size_label: "1.4 GB · fastest".to_string(),
            fits: r >= 4.0,
            note: "Smallest viable model. Curation quality drops noticeably.".to_string(),
        },
        ModelOption {
            model: "qwen3:4b".to_string(),
            display_name: "Qwen 3 · 4B".to_string(),
            size_label: "2.5 GB · balanced".to_string(),
            fits: r >= 12.0,
            note: "Sweet spot for 16 GB laptops.".to_string(),
        },
        ModelOption {
            model: "qwen3:8b".to_string(),
            display_name: "Qwen 3 · 8B".to_string(),
            size_label: "5.2 GB · best quality".to_string(),
            fits: profile.is_apple_silicon && r >= 16.0 || r >= 24.0,
            note: "Recommended on Apple Silicon and on 32 GB+ desktops.".to_string(),
        },
        ModelOption {
            model: "phi4-mini:3.8b".to_string(),
            display_name: "Phi-4 · 3.8B mini".to_string(),
            size_label: "2.3 GB · alternative".to_string(),
            fits: r >= 12.0,
            note: "Microsoft's instruction-tuned mini model — strong on JSON output.".to_string(),
        },
    ]
}

fn bytes_to_gb(b: u64) -> f64 {
    (b as f64) / (1024.0 * 1024.0 * 1024.0)
}

// ════════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_profile(ram_gb: f64, apple: bool) -> HardwareProfile {
        HardwareProfile {
            os: if apple { "macos" } else { "windows" }.to_string(),
            total_ram_gb: ram_gb,
            free_ram_gb: ram_gb * 0.5,
            cpu_brand: "Test CPU".to_string(),
            cpu_cores: 8,
            is_apple_silicon: apple,
        }
    }

    #[test]
    fn apple_silicon_16gb_recommends_8b() {
        let r = recommend(&mk_profile(16.0, true));
        assert_eq!(r.model, "qwen3:8b");
        assert_eq!(r.tier, "comfortable");
    }

    #[test]
    fn x86_32gb_recommends_8b() {
        let r = recommend(&mk_profile(32.0, false));
        assert_eq!(r.model, "qwen3:8b");
        assert_eq!(r.tier, "comfortable");
    }

    #[test]
    fn x86_16gb_recommends_4b() {
        let r = recommend(&mk_profile(16.0, false));
        assert_eq!(r.model, "qwen3:4b");
        assert_eq!(r.tier, "tight");
    }

    #[test]
    fn x86_15_92gb_still_recommends_4b() {
        // Windows on a "16 GB" laptop typically reports ~15.92 GB. Make sure
        // we round into the right tier rather than dumping to "minimum".
        let r = recommend(&mk_profile(15.92, false));
        assert_eq!(r.model, "qwen3:4b");
        assert_eq!(r.tier, "tight");
    }

    #[test]
    fn x86_31_5gb_recommends_8b() {
        // A nominal 32 GB machine often reports ~31.5 GB.
        let r = recommend(&mk_profile(31.5, false));
        assert_eq!(r.model, "qwen3:8b");
        assert_eq!(r.tier, "comfortable");
    }

    #[test]
    fn x86_8gb_recommends_minimum() {
        let r = recommend(&mk_profile(8.0, false));
        assert_eq!(r.model, "qwen3:1.7b");
        assert_eq!(r.tier, "minimum");
    }

    #[test]
    fn under_8gb_marks_unsupported() {
        let r = recommend(&mk_profile(4.0, false));
        assert_eq!(r.tier, "unsupported");
    }

    #[test]
    fn known_models_marks_8b_as_fitting_on_apple_16gb() {
        let opts = known_models(&mk_profile(16.0, true));
        let m8 = opts.iter().find(|o| o.model == "qwen3:8b").unwrap();
        assert!(m8.fits);
    }

    #[test]
    fn known_models_marks_8b_as_not_fitting_on_x86_16gb() {
        let opts = known_models(&mk_profile(16.0, false));
        let m8 = opts.iter().find(|o| o.model == "qwen3:8b").unwrap();
        assert!(!m8.fits);
    }

    #[test]
    fn detect_profile_returns_realistic_values() {
        // Smoke test against the actual host. Exact values vary, but
        // RAM and core count should be > 0.
        let p = detect_profile();
        assert!(p.total_ram_gb > 0.0);
        assert!(p.cpu_cores > 0);
        assert!(!p.os.is_empty());
    }
}
