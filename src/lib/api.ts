import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  CompetitionProfile,
  ConflictScanResult,
  EmailConfig,
  GenerateResult,
  HardwareProfile,
  LocalModelInfo,
  ModelOption,
  ModelRecommendation,
  NewsletterMeta,
  OllamaStatus,
  TopicRequest,
} from "./types";

/**
 * Fetch the application configuration from the Rust backend.
 */
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

/**
 * Persist the application configuration via the Rust backend.
 */
export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke<void>("save_config", { config });
}

/**
 * Kick off newsletter generation for one or more topics.
 * Progress events are emitted on the "newsletter://progress" channel.
 */
export async function generateNewsletter(
  topics: TopicRequest[],
  sources: string[],
  maxPapers: number,
  daysBack: number
): Promise<GenerateResult[]> {
  return invoke<GenerateResult[]>("generate_newsletter", {
    topics,
    sources,
    maxPapers,
    daysBack,
  });
}

/**
 * Scan the output directory and return metadata for all saved newsletters.
 */
export async function listNewsletters(
  outputDir: string
): Promise<NewsletterMeta[]> {
  return invoke<NewsletterMeta[]>("list_newsletters", { outputDir });
}

/**
 * Read the full Markdown content of a newsletter file.
 */
export async function readNewsletter(path: string): Promise<string> {
  return invoke<string>("read_newsletter", { path });
}

/**
 * Send a newsletter file via configured SMTP settings.
 */
export async function sendNewsletterEmail(path: string): Promise<void> {
  return invoke<void>("send_newsletter_email", { path });
}

/**
 * Open an SMTP connection with the given settings and close it without
 * sending. Validates host / port / TLS / auth in one round-trip.
 */
export async function testEmailConnection(
  emailConfig: EmailConfig
): Promise<void> {
  return invoke<void>("test_email_connection", { emailConfig });
}

// ─── Local AI ────────────────────────────────────────────────────────────────

/**
 * Detect the host machine's hardware profile (RAM, CPU, OS).
 */
export async function detectHardware(): Promise<HardwareProfile> {
  return invoke<HardwareProfile>("detect_hardware");
}

/**
 * Get a model recommendation for the current machine.
 */
export async function recommendLocalModel(): Promise<ModelRecommendation> {
  return invoke<ModelRecommendation>("recommend_local_model");
}

/**
 * List the static catalog of local models we know about, annotated for the
 * current machine (`fits` flag).
 */
export async function listKnownLocalModels(): Promise<ModelOption[]> {
  return invoke<ModelOption[]>("list_known_local_models");
}

/**
 * Probe Ollama at the given host. Returns running=false if it is unreachable
 * (does not throw — UI can treat it as a status).
 */
export async function checkOllamaStatus(host: string): Promise<OllamaStatus> {
  return invoke<OllamaStatus>("check_ollama_status", { host });
}

/**
 * List the models currently installed in the Ollama instance at host.
 */
export async function listInstalledLocalModels(
  host: string
): Promise<LocalModelInfo[]> {
  return invoke<LocalModelInfo[]>("list_installed_local_models", { host });
}

/**
 * Pull a model into Ollama. Streams progress on the `local-pull-progress`
 * event channel; await the returned promise for completion.
 */
export async function pullLocalModel(
  host: string,
  model: string
): Promise<void> {
  return invoke<void>("pull_local_model", { host, model });
}

/**
 * Cancel any in-flight model pull. Idempotent — safe to call when no pull
 * is active.
 */
export async function cancelLocalPull(): Promise<void> {
  return invoke<void>("cancel_local_pull");
}

/**
 * Returns the model name currently being pulled, or null if none.
 * Used at app startup to restore the pull-progress UI if a pull was
 * left in flight when the app last rendered.
 */
export async function getActivePull(): Promise<{ model: string | null }> {
  return invoke<{ model: string | null }>("get_active_pull");
}

/**
 * Run the conflict scanner for the given profiles against specified sources.
 */
export async function scanConflicts(
  profiles: CompetitionProfile[],
  sources: string[],
  maxPapers: number,
  daysBack: number,
): Promise<ConflictScanResult[]> {
  return invoke("scan_conflicts", {
    profiles,
    sources,
    max_papers: maxPapers,
    days_back: daysBack,
  });
}
