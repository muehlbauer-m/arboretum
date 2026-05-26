import { useState, useEffect, useCallback } from "react";
import {
  Save,
  Loader,
  CheckCircle,
  AlertCircle,
  Eye,
  EyeOff,
  Plus,
  X,
  RefreshCw,
  Sun,
  Moon,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getConfig, saveConfig, testEmailConnection } from "../lib/api";
import { useTheme, type Theme } from "../lib/theme";
import type { AppConfig, EmailConfig, TopicRequest } from "../lib/types";
import Serif from "../components/Serif";
import SmallCaps from "../components/SmallCaps";
import WoodRings from "../components/WoodRings";
import EmailSetupWizard from "../components/EmailSetupWizard";
import LocalAiSetup from "../components/LocalAiSetup";

// ─── Windows Task Scheduler API ──────────────────────────────────────────────

interface TaskStatus {
  exists: boolean;
  next_run: string | null;
  status: string | null;
}

const createSchedule = (config: AppConfig) =>
  invoke("create_schedule", { config });
const deleteSchedule = () => invoke("delete_schedule");
const getScheduleStatus = () => invoke<TaskStatus>("get_schedule_status");

// ─── Section primitive ──────────────────────────────────────────────────────

interface SettingsSectionProps {
  title: string;
  description: string;
  children: React.ReactNode;
}

function SettingsSection({ title, description, children }: SettingsSectionProps) {
  return (
    <section className="grid grid-cols-[220px_1fr] gap-8 py-6 border-b border-rule-soft last:border-b-0">
      <div>
        <Serif as="h3" size={15} weight={500}>
          {title}
        </Serif>
        <p className="text-[12px] text-ink-muted mt-1.5 leading-[1.5]">
          {description}
        </p>
      </div>
      <div className="min-w-0">{children}</div>
    </section>
  );
}

// ─── Toggle primitive ───────────────────────────────────────────────────────

function Toggle({
  on,
  onChange,
  disabled,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={() => !disabled && onChange(!on)}
      disabled={disabled}
      aria-pressed={on}
      className={`
        relative w-9 h-5 p-0 border-0 rounded-full shrink-0
        transition-colors duration-150
        ${on ? "bg-pine" : "bg-rule"}
        disabled:opacity-50 disabled:cursor-not-allowed
      `}
    >
      <span
        className="
          absolute top-0.5 w-4 h-4 bg-surface rounded-full shadow
          transition-[left] duration-150
        "
        style={{ left: on ? 18 : 2 }}
      />
    </button>
  );
}

// ─── Page ───────────────────────────────────────────────────────────────────

export default function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [taskStatus, setTaskStatus] = useState<TaskStatus | null>(null);
  const [taskStatusLoading, setTaskStatusLoading] = useState(false);
  const [theme, setTheme] = useTheme();
  const [testingEmail, setTestingEmail] = useState(false);
  const [emailTestResult, setEmailTestResult] = useState<
    { ok: true } | { ok: false; error: string } | null
  >(null);

  const refreshTaskStatus = useCallback(async () => {
    setTaskStatusLoading(true);
    try {
      const status = await getScheduleStatus();
      setTaskStatus(status);
    } catch {
      setTaskStatus(null);
    } finally {
      setTaskStatusLoading(false);
    }
  }, []);

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
    refreshTaskStatus();
  }, [refreshTaskStatus]);

  const update = <K extends keyof AppConfig>(key: K, val: AppConfig[K]) => {
    setConfig((prev) => (prev ? { ...prev, [key]: val } : prev));
  };

  const updateEmail = <K extends keyof AppConfig["email"]>(
    key: K,
    val: AppConfig["email"][K]
  ) => {
    setConfig((prev) =>
      prev ? { ...prev, email: { ...prev.email, [key]: val } } : prev
    );
    // Any edit invalidates a previous test result.
    setEmailTestResult(null);
  };

  const patchEmail = (patch: Partial<EmailConfig>) => {
    setConfig((prev) =>
      prev ? { ...prev, email: { ...prev.email, ...patch } } : prev
    );
    setEmailTestResult(null);
  };

  const patchLocalLlm = (patch: Partial<AppConfig["local_llm"]>) => {
    setConfig((prev) =>
      prev ? { ...prev, local_llm: { ...prev.local_llm, ...patch } } : prev
    );
  };

  const handleTestEmail = async () => {
    if (!config) return;
    setTestingEmail(true);
    setEmailTestResult(null);
    try {
      await testEmailConnection(config.email);
      // Persist the verified credentials so the Send path (which reads
      // from the saved config, not the form state) uses what we just
      // tested. Otherwise a successful test followed by Send would fail
      // with stale credentials.
      try {
        await saveConfig(config);
      } catch (saveErr) {
        setEmailTestResult({
          ok: false,
          error:
            `Connection verified but save failed — ${String(saveErr)}. ` +
            "Send won't work until you click Save changes.",
        });
        return;
      }
      setEmailTestResult({ ok: true });
    } catch (e) {
      setEmailTestResult({ ok: false, error: String(e) });
    } finally {
      setTestingEmail(false);
    }
  };

  const updateSchedule = <K extends keyof AppConfig["schedule"]>(
    key: K,
    val: AppConfig["schedule"][K]
  ) => {
    setConfig((prev) =>
      prev ? { ...prev, schedule: { ...prev.schedule, [key]: val } } : prev
    );
  };

  const addScheduleTopic = () => {
    if (!config) return;
    updateSchedule("topics", [
      ...config.schedule.topics,
      { query: "" },
    ] as TopicRequest[]);
  };

  const removeScheduleTopic = (i: number) => {
    if (!config) return;
    const t = [...config.schedule.topics];
    t.splice(i, 1);
    updateSchedule("topics", t);
  };

  const updateScheduleTopic = (i: number, q: string) => {
    if (!config) return;
    const t = [...config.schedule.topics];
    t[i] = { query: q };
    updateSchedule("topics", t);
  };

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      await saveConfig(config);
      if (config.schedule.enabled) {
        await createSchedule(config);
      } else {
        await deleteSchedule().catch(() => undefined);
      }
      await refreshTaskStatus();
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center gap-3 text-ink-muted">
        <Loader size={20} className="animate-spin" />
        Loading settings…
      </div>
    );
  }

  if (!config) {
    return (
      <div className="h-full flex items-center justify-center gap-3 text-rust">
        <AlertCircle size={20} />
        Failed to load config
      </div>
    );
  }

  const provider = config.ai_provider ?? "gemini";

  return (
    <div className="h-full flex flex-col overflow-hidden bg-canvas">
      {/* Hero */}
      <div className="relative px-10 pt-10 pb-6 shrink-0 border-b border-rule-soft overflow-hidden">
        <div className="absolute right-[-60px] top-[-30px] text-pine">
          <WoodRings
            width={360}
            height={220}
            cx={180}
            cy={110}
            rings={12}
            opacity={0.05}
            seed={9}
          />
        </div>
        <div className="relative flex items-center justify-between gap-6">
          <div>
            <SmallCaps>Preferences</SmallCaps>
            <Serif
              as="h1"
              italic
              weight={400}
              size={32}
              className="mt-2 leading-[1.1]"
            >
              Settings.
            </Serif>
          </div>
          <button
            onClick={handleSave}
            disabled={saving}
            className="btn-primary text-sm"
          >
            {saving ? (
              <Loader size={14} className="animate-spin" />
            ) : saved ? (
              <CheckCircle size={14} />
            ) : (
              <Save size={14} />
            )}
            {saving ? "Saving…" : saved ? "Saved" : "Save changes"}
          </button>
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-10 mt-4 flex items-center gap-2 p-3 bg-rust/10 border border-rust/25 rounded-card text-rust text-sm">
          <AlertCircle size={15} />
          {error}
        </div>
      )}

      {/* Body */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-[820px] mx-auto px-10 py-4">
          {/* AI Provider */}
          <SettingsSection
            title="AI provider"
            description="Choose how papers are curated and summarized."
          >
            <div className="grid grid-cols-3 gap-2.5">
              {[
                {
                  id: "claude",
                  label: "Claude (Anthropic)",
                  sub: "Sonnet 4.6 · API key required",
                },
                {
                  id: "gemini",
                  label: "Google Gemini",
                  sub: "Flash 2.5 · API key required",
                },
                {
                  id: "local",
                  label: "Local (Ollama)",
                  sub: "Offline · runs on your machine",
                },
              ].map((o) => {
                const active = provider === o.id;
                return (
                  <button
                    key={o.id}
                    onClick={() => update("ai_provider", o.id)}
                    className={`
                      text-left px-3.5 py-3 rounded-[10px] border transition-colors duration-150
                      ${
                        active
                          ? "bg-sage-wash border-moss"
                          : "bg-surface border-rule hover:border-ink-faint"
                      }
                    `}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span
                        className={`
                          inline-block w-3.5 h-3.5 rounded-full border-2
                          ${active ? "border-pine bg-pine" : "border-rule bg-surface"}
                        `}
                      >
                        {active && (
                          <span className="block w-1 h-1 rounded-full bg-surface m-auto mt-[3px]" />
                        )}
                      </span>
                      <Serif size={14} weight={500}>
                        {o.label}
                      </Serif>
                    </div>
                    <div className="pl-[22px] text-[11.5px] text-ink-muted">
                      {o.sub}
                    </div>
                  </button>
                );
              })}
            </div>

            {provider === "claude" && (
              <div className="mt-4">
                <label className="label">Anthropic API key</label>
                <div className="relative">
                  <input
                    type={showKey ? "text" : "password"}
                    value={config.claude_api_key}
                    onChange={(e) => update("claude_api_key", e.target.value)}
                    placeholder="sk-ant-…"
                    className="input pr-10"
                  />
                  <button
                    onClick={() => setShowKey(!showKey)}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-ink-faint hover:text-ink-soft"
                  >
                    {showKey ? <EyeOff size={15} /> : <Eye size={15} />}
                  </button>
                </div>
                <p className="text-[11.5px] text-ink-faint mt-1.5">
                  Get your key from the{" "}
                  <a
                    href="https://console.anthropic.com/settings/keys"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-pine underline underline-offset-2 decoration-pine/40 hover:decoration-pine"
                  >
                    Anthropic Console
                  </a>
                  . Stored locally in the OS credential store.
                </p>
              </div>
            )}

            {provider === "gemini" && (
              <div className="mt-4">
                <label className="label">Gemini API key</label>
                <div className="relative">
                  <input
                    type={showKey ? "text" : "password"}
                    value={config.gemini_api_key}
                    onChange={(e) => update("gemini_api_key", e.target.value)}
                    placeholder="AIza…"
                    className="input pr-10"
                  />
                  <button
                    onClick={() => setShowKey(!showKey)}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-ink-faint hover:text-ink-soft"
                  >
                    {showKey ? <EyeOff size={15} /> : <Eye size={15} />}
                  </button>
                </div>
                <p className="text-[11.5px] text-ink-faint mt-1.5">
                  Get your key from{" "}
                  <a
                    href="https://aistudio.google.com/apikey"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-pine underline underline-offset-2 decoration-pine/40 hover:decoration-pine"
                  >
                    Google AI Studio
                  </a>
                </p>
              </div>
            )}

            {provider === "local" && (
              <div className="mt-5">
                <LocalAiSetup
                  config={config.local_llm}
                  onChange={patchLocalLlm}
                />
              </div>
            )}
          </SettingsSection>

          {/* Output */}
          <SettingsSection
            title="Output"
            description="Where finished newsletters are written on disk."
          >
            <label className="label">Output directory</label>
            <input
              type="text"
              value={config.output_dir}
              onChange={(e) => update("output_dir", e.target.value)}
              placeholder="C:\Users\…\Documents\newsletters"
              className="input"
            />
          </SettingsSection>

          {/* Defaults */}
          <SettingsSection
            title="Defaults"
            description="Starting values for each generation."
          >
            <div className="space-y-4">
              <div>
                <label className="label">Sources</label>
                <div className="flex gap-2">
                  {["openalex", "arxiv"].map((src) => {
                    const active = config.default_sources.includes(src);
                    return (
                      <button
                        key={src}
                        onClick={() =>
                          update(
                            "default_sources",
                            active
                              ? config.default_sources.filter((s) => s !== src)
                              : [...config.default_sources, src]
                          )
                        }
                        className={`
                          px-3 py-1.5 rounded-md text-[12.5px] font-medium border transition-colors duration-150
                          ${
                            active
                              ? "bg-sage-wash border-moss text-pine-deep"
                              : "bg-surface border-rule text-ink-muted hover:border-ink-faint hover:text-ink-soft"
                          }
                        `}
                      >
                        {src === "openalex" ? "OpenAlex" : "arXiv"}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="label">Max papers / source</label>
                  <div className="flex items-center gap-3">
                    <input
                      type="range"
                      min={10}
                      max={100}
                      step={10}
                      value={config.default_max_papers}
                      onChange={(e) =>
                        update("default_max_papers", Number(e.target.value))
                      }
                      className="flex-1 accent-pine"
                    />
                    <span className="w-8 text-right text-[12px] text-ink">
                      {config.default_max_papers}
                    </span>
                  </div>
                </div>
                <div>
                  <label className="label">Days back</label>
                  <select
                    value={config.default_days_back}
                    onChange={(e) =>
                      update("default_days_back", Number(e.target.value))
                    }
                    className="input py-1.5 text-[12.5px]"
                  >
                    {[7, 14, 30, 60, 90, 180, 365].map((d) => (
                      <option key={d} value={d}>
                        {d} days
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </div>
          </SettingsSection>

          {/* Email */}
          <SettingsSection
            title="Email delivery"
            description="Send completed newsletters over SMTP. Pick your provider first — we'll pre-fill host and port."
          >
            <div className="flex items-center justify-between mb-5">
              <span className="text-[13px] text-ink-soft">
                {config.email.enabled ? "Enabled" : "Disabled"}
              </span>
              <Toggle
                on={config.email.enabled}
                onChange={(v) => updateEmail("enabled", v)}
              />
            </div>

            <div
              className={
                !config.email.enabled ? "opacity-50 pointer-events-none" : ""
              }
            >
              <EmailSetupWizard
                email={config.email}
                onChange={patchEmail}
                onTest={handleTestEmail}
                testing={testingEmail}
                testResult={emailTestResult}
              />
            </div>
          </SettingsSection>

          {/* Schedule */}
          <SettingsSection
            title="Schedule"
            description="Run unattended at a chosen cadence via the OS scheduler (Task Scheduler on Windows, launchd on macOS)."
          >
            <div className="flex items-center justify-between mb-4">
              <span className="text-[13px] text-ink-soft">
                {config.schedule.enabled ? "Running on schedule" : "Off"}
              </span>
              <Toggle
                on={config.schedule.enabled}
                onChange={(v) => updateSchedule("enabled", v)}
              />
            </div>

            <div
              className={`
                space-y-4
                ${!config.schedule.enabled ? "opacity-50 pointer-events-none" : ""}
              `}
            >
              <div>
                <label className="label">Frequency</label>
                <div className="flex gap-2">
                  {(["daily", "weekly"] as const).map((freq) => {
                    const active = config.schedule.frequency === freq;
                    return (
                      <button
                        key={freq}
                        onClick={() => updateSchedule("frequency", freq)}
                        className={`
                          px-3 py-1.5 rounded-md text-[12.5px] font-medium border transition-colors duration-150
                          ${
                            active
                              ? "bg-sage-wash border-moss text-pine-deep"
                              : "bg-surface border-rule text-ink-muted hover:border-ink-faint hover:text-ink-soft"
                          }
                        `}
                      >
                        {freq === "daily" ? "Daily" : "Weekly"}
                      </button>
                    );
                  })}
                </div>
              </div>

              {config.schedule.frequency === "weekly" && (
                <div>
                  <label className="label">Days</label>
                  <div className="flex gap-1.5 flex-wrap">
                    {["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"].map(
                      (day) => {
                        const active = config.schedule.days.includes(day);
                        return (
                          <button
                            key={day}
                            onClick={() => {
                              const newDays = active
                                ? config.schedule.days.filter((d) => d !== day)
                                : [...config.schedule.days, day];
                              if (newDays.length > 0) {
                                updateSchedule("days", newDays);
                              }
                            }}
                            className={`
                              w-10 h-8 rounded-md text-[11px] font-medium border transition-colors duration-150
                              ${
                                active
                                  ? "bg-sage-wash border-moss text-pine-deep"
                                  : "bg-surface border-rule text-ink-muted hover:border-ink-faint hover:text-ink-soft"
                              }
                            `}
                          >
                            {day}
                          </button>
                        );
                      }
                    )}
                  </div>
                </div>
              )}

              <div>
                <label className="label">Time</label>
                <div className="flex items-center gap-2">
                  <select
                    value={config.schedule.time.split(":")[0] || "08"}
                    onChange={(e) => {
                      const mins =
                        config.schedule.time.split(":")[1] || "00";
                      updateSchedule("time", `${e.target.value}:${mins}`);
                    }}
                    className="input py-1.5 w-20"
                  >
                    {Array.from({ length: 24 }, (_, h) =>
                      String(h).padStart(2, "0")
                    ).map((h) => (
                      <option key={h} value={h}>
                        {h}
                      </option>
                    ))}
                  </select>
                  <span className="text-ink-muted font-serif">:</span>
                  <select
                    value={config.schedule.time.split(":")[1] || "00"}
                    onChange={(e) => {
                      const hrs = config.schedule.time.split(":")[0] || "08";
                      updateSchedule("time", `${hrs}:${e.target.value}`);
                    }}
                    className="input py-1.5 w-20"
                  >
                    {["00", "15", "30", "45"].map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                  <span className="text-[11px] text-ink-faint ml-1">
                    24-hour
                  </span>
                </div>
              </div>

              <div>
                <div className="flex items-center justify-between mb-2">
                  <label className="label mb-0">Scheduled topics</label>
                  <button
                    onClick={addScheduleTopic}
                    className="btn-ghost text-[11px] py-0.5"
                  >
                    <Plus size={11} />
                    Add
                  </button>
                </div>

                {config.schedule.topics.length === 0 ? (
                  <p className="text-[11.5px] text-ink-faint py-2.5 text-center">
                    No topics — add at least one for scheduled runs.
                  </p>
                ) : (
                  <div className="space-y-2">
                    {config.schedule.topics.map((t, i) => (
                      <div key={i} className="flex gap-2">
                        <input
                          type="text"
                          value={t.query}
                          onChange={(e) =>
                            updateScheduleTopic(i, e.target.value)
                          }
                          placeholder="Research interest…"
                          className="input flex-1"
                        />
                        <button
                          onClick={() => removeScheduleTopic(i)}
                          className="p-2 text-ink-faint hover:text-rust transition-colors"
                        >
                          <X size={14} />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Task status */}
            <div className="mt-5 card bg-surface-raised">
              <div className="flex items-center justify-between mb-2.5">
                <Serif size={14} weight={500}>
                  Scheduled task status
                </Serif>
                <button
                  onClick={refreshTaskStatus}
                  disabled={taskStatusLoading}
                  className="btn-ghost text-[11px] py-0.5"
                >
                  <RefreshCw
                    size={12}
                    className={taskStatusLoading ? "animate-spin" : ""}
                  />
                  Refresh
                </button>
              </div>

              <div className="space-y-1.5 text-[12.5px]">
                <div className="flex items-center gap-2">
                  <span className="text-ink-muted w-24 shrink-0">Task:</span>
                  {taskStatus?.exists ? (
                    <span className="text-success flex items-center gap-1.5">
                      <CheckCircle size={13} />
                      Registered
                    </span>
                  ) : (
                    <span className="text-ink-faint">Not registered</span>
                  )}
                </div>

                {taskStatus?.exists && taskStatus.status && (
                  <div className="flex items-center gap-2">
                    <span className="text-ink-muted w-24 shrink-0">
                      Status:
                    </span>
                    <span className="text-ink-soft">{taskStatus.status}</span>
                  </div>
                )}

                {taskStatus?.exists && taskStatus.next_run && (
                  <div className="flex items-center gap-2">
                    <span className="text-ink-muted w-24 shrink-0">
                      Next run:
                    </span>
                    <span className="text-ink-soft">
                      {taskStatus.next_run}
                    </span>
                  </div>
                )}

                {!taskStatus?.exists && (
                  <p className="text-[11.5px] text-ink-faint mt-1.5">
                    Enable the schedule and save to register a task with the
                    OS scheduler. The app will launch automatically at the
                    configured time.
                  </p>
                )}
              </div>
            </div>
          </SettingsSection>

          {/* Appearance */}
          <SettingsSection
            title="Appearance"
            description="Pick the palette you want to read in."
          >
            <div className="grid grid-cols-2 gap-2.5">
              {(
                [
                  {
                    id: "paper",
                    label: "Paper",
                    sub: "Warm ivory canvas",
                    Icon: Sun,
                  },
                  {
                    id: "forest",
                    label: "Forest",
                    sub: "Deep-night grove",
                    Icon: Moon,
                  },
                ] as { id: Theme; label: string; sub: string; Icon: typeof Sun }[]
              ).map(({ id, label, sub, Icon }) => {
                const active = theme === id;
                return (
                  <button
                    key={id}
                    onClick={() => setTheme(id)}
                    className={`
                      text-left px-3.5 py-3 rounded-[10px] border transition-colors duration-150
                      ${
                        active
                          ? "bg-sage-wash border-moss"
                          : "bg-surface border-rule hover:border-ink-faint"
                      }
                    `}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <Icon size={14} className="text-pine" strokeWidth={1.7} />
                      <Serif size={14} weight={500}>
                        {label}
                      </Serif>
                    </div>
                    <div className="pl-[22px] text-[11.5px] text-ink-muted">
                      {sub}
                    </div>
                  </button>
                );
              })}
            </div>
          </SettingsSection>
        </div>
      </div>
    </div>
  );
}
