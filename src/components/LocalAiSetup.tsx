import { useEffect, useState, useCallback } from "react";
import {
  CheckCircle,
  AlertCircle,
  Loader,
  Download,
  Sparkles,
  RefreshCw,
  ExternalLink,
  X,
} from "lucide-react";
import {
  detectHardware,
  recommendLocalModel,
  listKnownLocalModels,
  checkOllamaStatus,
  listInstalledLocalModels,
} from "../lib/api";
import type {
  HardwareProfile,
  LocalLlmConfig,
  LocalModelInfo,
  ModelOption,
  ModelRecommendation,
  OllamaStatus,
} from "../lib/types";
import Serif from "./Serif";
import SmallCaps from "./SmallCaps";
import { useLocalPull } from "../lib/LocalPullContext";

interface Props {
  config: LocalLlmConfig;
  onChange: (patch: Partial<LocalLlmConfig>) => void;
}

const TIER_COPY: Record<ModelRecommendation["tier"], { label: string; tone: string }> = {
  comfortable: { label: "Comfortable fit", tone: "text-success" },
  tight: { label: "Tight fit", tone: "text-amber-700" },
  minimum: { label: "Minimum spec", tone: "text-amber-700" },
  unsupported: { label: "Below recommended", tone: "text-rust" },
};

export default function LocalAiSetup({ config, onChange }: Props) {
  const [hw, setHw] = useState<HardwareProfile | null>(null);
  const [recommendation, setRecommendation] = useState<ModelRecommendation | null>(null);
  const [knownModels, setKnownModels] = useState<ModelOption[]>([]);
  const [installed, setInstalled] = useState<LocalModelInfo[]>([]);
  const [ollama, setOllama] = useState<OllamaStatus | null>(null);
  const [statusLoading, setStatusLoading] = useState(false);

  // Pull state lives in LocalPullContext so it survives leaving Settings.
  const {
    activeModel,
    progress: pullStatus,
    error: pullError,
    startPull,
    cancelPull,
  } = useLocalPull();

  const refreshOllama = useCallback(async (host: string) => {
    setStatusLoading(true);
    try {
      const status = await checkOllamaStatus(host);
      setOllama(status);
      if (status.running) {
        try {
          const models = await listInstalledLocalModels(host);
          setInstalled(models);
        } catch {
          setInstalled([]);
        }
      } else {
        setInstalled([]);
      }
    } finally {
      setStatusLoading(false);
    }
  }, []);

  // Detect hardware once
  useEffect(() => {
    detectHardware().then(setHw).catch(console.error);
    recommendLocalModel().then(setRecommendation).catch(console.error);
    listKnownLocalModels().then(setKnownModels).catch(console.error);
  }, []);

  // Status check whenever host changes
  useEffect(() => {
    refreshOllama(config.host);
  }, [config.host, refreshOllama]);

  // Refresh installed-models list whenever a pull finishes (activeModel
  // transitions back to null).
  useEffect(() => {
    if (activeModel === null && ollama?.running) {
      refreshOllama(config.host);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeModel]);

  const isInstalled = (modelId: string) =>
    installed.some((m) => m.name === modelId || m.name.startsWith(modelId + ":"));

  const handlePull = (modelId: string) => {
    void startPull(config.host, modelId);
  };

  const pullPercent = (() => {
    if (!pullStatus) return null;
    if (!pullStatus.total || !pullStatus.completed) return null;
    return Math.round((pullStatus.completed / pullStatus.total) * 100);
  })();

  return (
    <div className="space-y-5">
      {/* Hardware card */}
      <div className="card bg-surface-raised">
        <div className="flex items-center gap-2 mb-2">
          <Sparkles size={14} className="text-pine" />
          <Serif size={14} weight={500}>
            This machine
          </Serif>
        </div>
        {hw ? (
          <div className="grid grid-cols-2 gap-x-6 gap-y-1 text-[12.5px]">
            <Row label="OS" value={`${hw.os}${hw.is_apple_silicon ? " (Apple Silicon)" : ""}`} />
            <Row label="Total RAM" value={`${hw.total_ram_gb.toFixed(1)} GB`} />
            <Row label="CPU" value={hw.cpu_brand || "—"} />
            <Row label="Cores" value={String(hw.cpu_cores)} />
          </div>
        ) : (
          <div className="text-ink-faint text-[12px]">Detecting…</div>
        )}
      </div>

      {/* Recommendation banner */}
      {recommendation && (
        <div className="card bg-sage-wash border-moss">
          <div className="flex items-center gap-2 flex-wrap">
            <SmallCaps>Recommended</SmallCaps>
            <span className="text-rule">|</span>
            <SmallCaps>
              <span className={TIER_COPY[recommendation.tier].tone}>
                {TIER_COPY[recommendation.tier].label}
              </span>
            </SmallCaps>
          </div>
          <Serif size={16} weight={500} className="mt-1.5">
            {recommendation.display_name}
          </Serif>
          <p className="text-[12.5px] text-ink-soft mt-1.5 leading-[1.55]">
            {recommendation.reason}
          </p>
          <div className="text-[11.5px] text-ink-muted mt-2">
            ~{recommendation.estimated_disk_gb.toFixed(1)} GB on disk · ~
            {recommendation.estimated_ram_gb.toFixed(1)} GB working memory
          </div>
          {config.model !== recommendation.model && (
            <button
              onClick={() => onChange({ model: recommendation.model })}
              className="btn-ghost text-[11.5px] mt-2.5 py-0.5"
            >
              Use this model
            </button>
          )}
        </div>
      )}

      {/* Ollama status */}
      <div
        className={`card ${
          ollama?.running ? "bg-surface-raised" : "bg-amber-50 border-amber-200"
        }`}
      >
        <div className="flex items-center justify-between mb-3">
          <Serif size={14} weight={500}>
            Ollama
          </Serif>
          <button
            onClick={() => refreshOllama(config.host)}
            disabled={statusLoading}
            className="btn-ghost text-[11px] py-0.5"
            title="Re-check Ollama status"
          >
            <RefreshCw size={12} className={statusLoading ? "animate-spin" : ""} />
            Re-check
          </button>
        </div>

        <div className="flex items-center gap-2 text-[12.5px] mb-2.5">
          {ollama?.running ? (
            <>
              <CheckCircle size={14} className="text-success" />
              <span className="text-ink-soft">
                Reachable · v{ollama.version ?? "?"}
              </span>
            </>
          ) : (
            <>
              <AlertCircle size={14} className="text-rust" />
              <span className="text-ink-soft">
                Not reachable at {config.host}
              </span>
            </>
          )}
        </div>

        {!ollama?.running && (
          <div className="mb-3 p-3 bg-surface rounded-md border border-rule-soft">
            <Serif size={13} weight={500} className="mb-1">
              Local AI requires Ollama
            </Serif>
            <p className="text-[12px] text-ink-soft leading-[1.55] mb-2">
              Ollama is a separate, free service that hosts the language model
              for you. Arboretum doesn't bundle it — install it once, then
              come back to this page.
            </p>
            <a
              href="https://ollama.com/download"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-pine font-medium text-[12px] underline underline-offset-2 decoration-pine/40 hover:decoration-pine"
            >
              Download Ollama
              <ExternalLink size={11} />
            </a>
            <p className="text-[10.5px] text-ink-faint mt-2">
              ~700 MB installer · runs as a background service on port 11434
              · uninstall anytime via Add/Remove Programs.
            </p>
          </div>
        )}

        <label className="label">Ollama host</label>
        <input
          type="text"
          value={config.host}
          onChange={(e) => onChange({ host: e.target.value })}
          placeholder="http://127.0.0.1:11434"
          className="input"
        />
        <p className="text-[10.5px] text-ink-faint mt-1">
          Default talks to a local install. Change this to point at a remote
          Ollama (e.g. on your home server).
        </p>
      </div>

      {/* Model picker */}
      <div>
        <label className="label">Model</label>
        <div className="space-y-2">
          {knownModels.map((m) => {
            const isActive = config.model === m.model;
            const installedFlag = isInstalled(m.model);
            const recommended = recommendation?.model === m.model;
            const pullingThis = activeModel === m.model;
            return (
              <div
                key={m.model}
                className={`
                  flex items-center gap-3 px-3.5 py-3 rounded-[10px] border transition-colors duration-150
                  ${
                    isActive
                      ? "bg-sage-wash border-moss"
                      : "bg-surface border-rule hover:border-ink-faint"
                  }
                `}
              >
                <button
                  onClick={() => onChange({ model: m.model })}
                  className="flex-1 text-left flex items-start gap-2.5"
                >
                  <span
                    className={`
                      mt-0.5 inline-block w-3.5 h-3.5 rounded-full border-2 shrink-0
                      ${isActive ? "border-pine bg-pine" : "border-rule bg-surface"}
                    `}
                  >
                    {isActive && (
                      <span className="block w-1 h-1 rounded-full bg-surface m-auto mt-[3px]" />
                    )}
                  </span>
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <Serif size={14} weight={500}>
                        {m.display_name}
                      </Serif>
                      {recommended && (
                        <span className="px-1.5 py-0.5 rounded bg-pine text-surface text-[9.5px] font-semibold tracking-wider uppercase">
                          Recommended
                        </span>
                      )}
                      {installedFlag && (
                        <span className="text-[10px] text-success uppercase tracking-wider font-semibold">
                          ✓ Installed
                        </span>
                      )}
                      {!m.fits && !installedFlag && (
                        <span className="text-[10px] text-amber-700 uppercase tracking-wider font-semibold">
                          May be tight
                        </span>
                      )}
                    </div>
                    <div className="text-[11.5px] text-ink-muted mt-0.5">
                      {m.size_label} — {m.note}
                    </div>
                  </div>
                </button>
                {!installedFlag && ollama?.running && (
                  <button
                    onClick={() => handlePull(m.model)}
                    disabled={!!activeModel}
                    className="btn-ghost text-[11px] py-0.5 shrink-0"
                  >
                    {pullingThis ? (
                      <Loader size={11} className="animate-spin" />
                    ) : (
                      <Download size={11} />
                    )}
                    {pullingThis ? "Pulling…" : "Install"}
                  </button>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Pull progress */}
      {activeModel && (
        <div className="card bg-surface-raised">
          <div className="flex items-center gap-2 text-[12.5px] mb-2">
            <Loader size={13} className="animate-spin text-pine" />
            <span className="text-ink-soft flex-1">
              Pulling{" "}
              <code className="font-mono text-[12px]">{activeModel}</code>
              {pullStatus?.status ? ` · ${pullStatus.status}` : ""}
            </span>
            <button
              onClick={cancelPull}
              className="btn-ghost text-[11px] py-0.5 text-rust hover:text-rust"
              title="Cancel pull"
            >
              <X size={11} />
              Cancel
            </button>
          </div>
          {pullPercent !== null && (
            <div className="w-full bg-rule h-1.5 rounded-full overflow-hidden">
              <div
                className="h-full bg-pine transition-[width] duration-200"
                style={{ width: `${pullPercent}%` }}
              />
            </div>
          )}
          {pullStatus?.total && pullStatus.completed && (
            <div className="text-[10.5px] text-ink-faint mt-1">
              {(pullStatus.completed / 1e9).toFixed(2)} /{" "}
              {(pullStatus.total / 1e9).toFixed(2)} GB
            </div>
          )}
          <p className="text-[10.5px] text-ink-faint mt-2">
            Safe to navigate away — the pull continues and a small indicator
            shows in the sidebar.
          </p>
        </div>
      )}
      {pullError && (
        <div className="flex items-center gap-2 p-3 bg-rust/10 border border-rust/25 rounded-card text-rust text-sm">
          <AlertCircle size={15} />
          {pullError}
        </div>
      )}

      {/* Parameters */}
      <div>
        <label className="label">Tune parameters</label>
        <div className="space-y-3.5">
          <Slider
            label="Context window"
            help="Tokens the model can see at once. Bigger = more papers fit; eats RAM."
            min={4096}
            max={32768}
            step={2048}
            unit=" tok"
            value={config.num_ctx}
            onChange={(v) => onChange({ num_ctx: v })}
          />
          <Slider
            label="Curation creativity"
            help="0 = deterministic, 1 = lively. 0.7 is the upstream default."
            min={0}
            max={1.0}
            step={0.05}
            value={config.temperature_curation}
            onChange={(v) => onChange({ temperature_curation: v })}
            format={(v) => v.toFixed(2)}
          />
          <Slider
            label="Curation max output"
            help="Caps the curated newsletter length. ~3-4k is typical."
            min={1024}
            max={16384}
            step={512}
            unit=" tok"
            value={config.num_predict_curation}
            onChange={(v) => onChange({ num_predict_curation: v })}
          />
          <Slider
            label="Conflict scan creativity"
            help="Lower so the JSON output stays well-structured."
            min={0}
            max={1.0}
            step={0.05}
            value={config.temperature_scan}
            onChange={(v) => onChange({ temperature_scan: v })}
            format={(v) => v.toFixed(2)}
          />
          <Slider
            label="Scan max output"
            help="Caps the JSON output for conflict evaluation."
            min={2048}
            max={16384}
            step={512}
            unit=" tok"
            value={config.num_predict_scan}
            onChange={(v) => onChange({ num_predict_scan: v })}
          />
        </div>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <>
      <span className="text-ink-muted">{label}</span>
      <span className="text-ink-soft truncate">{value}</span>
    </>
  );
}

interface SliderProps {
  label: string;
  help: string;
  min: number;
  max: number;
  step: number;
  value: number;
  onChange: (v: number) => void;
  unit?: string;
  format?: (v: number) => string;
}

function Slider({
  label,
  help,
  min,
  max,
  step,
  value,
  onChange,
  unit = "",
  format,
}: SliderProps) {
  const display = format ? format(value) : `${value}${unit}`;
  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <label className="label mb-0">{label}</label>
        <span className="text-[11.5px] text-ink-soft tabular-nums">{display}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-pine"
      />
      <p className="text-[10.5px] text-ink-faint mt-1 leading-[1.5]">{help}</p>
    </div>
  );
}
