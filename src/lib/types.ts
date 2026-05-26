// ─── Config Types ────────────────────────────────────────────────────────────

export interface EmailConfig {
  enabled: boolean;
  smtp_host: string;
  smtp_port: number;
  smtp_user: string;
  smtp_password: string;
  recipient: string;
}

export interface ScheduleConfig {
  enabled: boolean;
  frequency: string;    // "daily" or "weekly"
  days: string[];       // e.g. ["MON", "WED", "FRI"] (for weekly)
  time: string;         // "08:00" (HH:MM 24h format)
  topics: TopicRequest[];
}

export interface LocalLlmConfig {
  host: string;
  model: string;
  num_ctx: number;
  num_predict_curation: number;
  num_predict_scan: number;
  temperature_curation: number;
  temperature_scan: number;
}

export interface AppConfig {
  gemini_api_key: string;
  claude_api_key: string;
  ai_provider: string;
  output_dir: string;
  default_sources: string[];
  default_max_papers: number;
  default_days_back: number;
  email: EmailConfig;
  schedule: ScheduleConfig;
  conflict_profiles: CompetitionProfile[];
  conflict_settings: ConflictSettings;
  local_llm: LocalLlmConfig;
}

// ─── Local AI Types ──────────────────────────────────────────────────────────

export interface HardwareProfile {
  os: string;
  total_ram_gb: number;
  free_ram_gb: number;
  cpu_brand: string;
  cpu_cores: number;
  is_apple_silicon: boolean;
}

export interface ModelRecommendation {
  model: string;
  display_name: string;
  reason: string;
  tier: "comfortable" | "tight" | "minimum" | "unsupported";
  estimated_disk_gb: number;
  estimated_ram_gb: number;
}

export interface ModelOption {
  model: string;
  display_name: string;
  size_label: string;
  fits: boolean;
  note: string;
}

export interface OllamaStatus {
  running: boolean;
  version: string | null;
  error: string | null;
}

export interface LocalModelInfo {
  name: string;
  size_bytes: number;
}

export interface LocalPullProgress {
  model: string;
  status: string;
  total: number | null;
  completed: number | null;
}

// ─── Request / Response Types ────────────────────────────────────────────────

export interface TopicRequest {
  query: string;
}

export interface GenerateResult {
  topic: string;
  title: string;
  path: string;
  error: string | null;
}

export interface NewsletterMeta {
  path: string;
  filename: string;
  title: string;
  date: string;
  size_kb: number;
}

// ─── Progress Event ───────────────────────────────────────────────────────────

export interface ProgressEvent {
  topic: string;
  step: string;
  message: string;
  done: boolean;
  error: boolean;
  // Optional streaming-progress fields (only populated when the local
  // provider is in use). Other providers leave these undefined.
  tokens_generated?: number;
  tokens_per_sec?: number;
  elapsed_ms?: number;
}

// ─── Conflict Scanner Types ──────────────────────────────────────────────────

export interface CompetitionProfile {
  id: string;
  name: string;
  research_description: string;
  key_terms: string[];
  own_papers: string[];
  enabled: boolean;
  last_scanned: string | null;
  last_overlap_count: number;
}

export interface ConflictSettings {
  max_papers_per_source: number;
  scan_days_back: number;
  competition_threshold: number;
  auto_scan_with_newsletter: boolean;
}

export interface OverlapResult {
  paper_title: string;
  paper_authors: string;
  paper_url: string;
  paper_date: string;
  paper_source: string;
  overlap_score: number;
  overlap_explanation: string;
  matched_terms: string[];
  profile_id: string;
  profile_name: string;
}

export interface ConflictScanResult {
  profile_id: string;
  profile_name: string;
  overlaps: OverlapResult[];
  scanned_at: string;
  papers_checked: number;
  error: string | null;
}

// ─── UI State ─────────────────────────────────────────────────────────────────

export type Page = "home" | "scanner" | "history" | "settings" | "help";

export interface TopicCard {
  id: string;
  query: string;
}

export interface LogEntry {
  ts: string;
  topic: string;
  message: string;
  type: "info" | "success" | "error";
  // When set, this entry represents in-progress streaming output and the
  // next streaming entry from the same step should *replace* it rather
  // than append. Keeps the log readable during long local-model runs.
  streamingStep?: string;
  tokens_generated?: number;
  tokens_per_sec?: number;
  elapsed_ms?: number;
}
