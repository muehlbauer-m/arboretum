// Type-level tests to ensure TypeScript interfaces match expected Rust serialization

import type { AppConfig, CompetitionProfile, ConflictSettings, GenerateResult, ProgressEvent, NewsletterMeta, LogEntry } from "./types";

describe("Type contracts", () => {
  it("AppConfig has required fields", () => {
    const config: AppConfig = {
      gemini_api_key: "",
      claude_api_key: "",
      ai_provider: "gemini",
      output_dir: "",
      default_sources: [],
      default_max_papers: 50,
      default_days_back: 90,
      email: {
        enabled: false,
        smtp_host: "",
        smtp_port: 587,
        smtp_user: "",
        smtp_password: "",
        recipient: "",
      },
      schedule: {
        enabled: false,
        frequency: "weekly",
        days: ["MON"],
        time: "08:00",
        topics: [],
      },
      conflict_profiles: [],
      conflict_settings: {
        max_papers_per_source: 200,
        scan_days_back: 30,
        competition_threshold: 30,
        auto_scan_with_newsletter: false,
      },
    };
    expect(config.default_max_papers).toBe(50);
    expect(config.email.smtp_port).toBe(587);
  });

  it("GenerateResult can have null error", () => {
    const result: GenerateResult = {
      topic: "test",
      title: "Test",
      path: "/path.md",
      error: null,
    };
    expect(result.error).toBeNull();
  });

  it("ProgressEvent has all required fields", () => {
    const event: ProgressEvent = {
      topic: "ml",
      step: "keywords",
      message: "Extracting…",
      done: false,
      error: false,
    };
    expect(event.done).toBe(false);
  });

  it("AppConfig conflict_profiles and conflict_settings are present", () => {
    const settings: ConflictSettings = {
      max_papers_per_source: 200,
      scan_days_back: 30,
      competition_threshold: 30,
      auto_scan_with_newsletter: false,
    };
    const profile: CompetitionProfile = {
      id: "p1",
      name: "Test Lab",
      research_description: "Neural networks",
      key_terms: ["deep learning", "transformers"],
      own_papers: [],
      enabled: true,
      last_scanned: null,
      last_overlap_count: 0,
    };
    expect(settings.max_papers_per_source).toBe(200);
    expect(settings.competition_threshold).toBe(30);
    expect(profile.enabled).toBe(true);
    expect(profile.last_scanned).toBeNull();
    expect(profile.key_terms).toHaveLength(2);
  });

  it("LogEntry type discriminant is correct", () => {
    const entries: LogEntry[] = [
      { ts: "00:00:00", topic: "", message: "info msg", type: "info" },
      { ts: "00:00:01", topic: "", message: "error msg", type: "error" },
      { ts: "00:00:02", topic: "", message: "ok", type: "success" },
    ];
    expect(entries.map((e) => e.type)).toEqual(["info", "error", "success"]);
  });
});
