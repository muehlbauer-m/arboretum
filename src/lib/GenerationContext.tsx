import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { generateNewsletter, getConfig } from "./api";
import type {
  TopicCard,
  LogEntry,
  ProgressEvent,
  GenerateResult,
  AppConfig,
} from "./types";

let _idCounter = 0;
function newId() {
  return `topic-${++_idCounter}-${Date.now()}`;
}

interface GenerationState {
  topics: TopicCard[];
  sources: string[];
  maxPapers: number;
  daysBack: number;
  running: boolean;
  log: LogEntry[];
  results: GenerateResult[];
  showOptions: boolean;
  config: AppConfig | null;
  addTopic: () => void;
  updateTopic: (id: string, value: string) => void;
  removeTopic: (id: string) => void;
  toggleSource: (src: string) => void;
  handleGenerate: () => void;
  setMaxPapers: (v: number) => void;
  setDaysBack: (v: number) => void;
  setShowOptions: (v: boolean) => void;
}

const GenerationContext = createContext<GenerationState | null>(null);

export function GenerationProvider({ children }: { children: ReactNode }) {
  const [topics, setTopics] = useState<TopicCard[]>([
    { id: newId(), query: "" },
  ]);
  const [sources, setSources] = useState<string[]>(["openalex", "arxiv"]);
  const [maxPapers, setMaxPapers] = useState(50);
  const [daysBack, setDaysBack] = useState(90);
  const [running, setRunning] = useState(false);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [results, setResults] = useState<GenerateResult[]>([]);
  const [showOptions, setShowOptions] = useState(false);
  const [config, setConfig] = useState<AppConfig | null>(null);

  // Load config defaults on mount
  useEffect(() => {
    getConfig()
      .then((cfg) => {
        setConfig(cfg);
        setSources(cfg.default_sources);
        setMaxPapers(cfg.default_max_papers);
        setDaysBack(cfg.default_days_back);
      })
      .catch(console.error);
  }, []);

  const addLog = useCallback(
    (
      topic: string,
      message: string,
      type: LogEntry["type"] = "info",
      streaming?: {
        step: string;
        tokens_generated?: number;
        tokens_per_sec?: number;
        elapsed_ms?: number;
      }
    ) => {
      const ts = new Date().toLocaleTimeString("en-US", {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
      setLog((prev) => {
        const entry: LogEntry = {
          ts,
          topic,
          message,
          type,
          streamingStep: streaming?.step,
          tokens_generated: streaming?.tokens_generated,
          tokens_per_sec: streaming?.tokens_per_sec,
          elapsed_ms: streaming?.elapsed_ms,
        };
        // Coalesce consecutive streaming events from the same step into
        // a single, in-place updating row. Non-streaming events break
        // the chain so the streamed snapshot is preserved as a regular
        // log line above.
        if (streaming && prev.length > 0) {
          const last = prev[prev.length - 1];
          if (last.streamingStep === streaming.step) {
            return [...prev.slice(0, -1), entry];
          }
        }
        return [...prev, entry];
      });
    },
    []
  );

  const addTopic = useCallback(() => {
    setTopics((prev) => [...prev, { id: newId(), query: "" }]);
  }, []);

  const updateTopic = useCallback((id: string, value: string) => {
    setTopics((prev) =>
      prev.map((t) => (t.id === id ? { ...t, query: value } : t))
    );
  }, []);

  const removeTopic = useCallback((id: string) => {
    setTopics((prev) => {
      if (prev.length <= 1) return prev;
      return prev.filter((t) => t.id !== id);
    });
  }, []);

  const toggleSource = useCallback((src: string) => {
    setSources((prev) =>
      prev.includes(src) ? prev.filter((s) => s !== src) : [...prev, src]
    );
  }, []);

  const handleGenerate = useCallback(async () => {
    const validTopics = topics.filter((t) => t.query.trim());
    if (validTopics.length === 0) {
      addLog("", "Please enter at least one topic.", "error");
      return;
    }
    if (sources.length === 0) {
      addLog("", "Please select at least one source.", "error");
      return;
    }

    let currentConfig = config;
    try {
      currentConfig = await getConfig();
      setConfig(currentConfig);
    } catch (e) {
      addLog("", `Failed to load config: ${String(e)}`, "error");
      return;
    }

    const provider = currentConfig?.ai_provider ?? "gemini";
    if (provider === "gemini" && !currentConfig?.gemini_api_key) {
      addLog(
        "",
        "Gemini API key not configured. Go to Settings to add it.",
        "error"
      );
      return;
    }

    setRunning(true);
    setResults([]);
    setLog([]);

    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<ProgressEvent>(
        "newsletter-progress",
        (event) => {
          const p = event.payload;
          const streaming =
            p.tokens_generated !== undefined
              ? {
                  step: p.step,
                  tokens_generated: p.tokens_generated,
                  tokens_per_sec: p.tokens_per_sec,
                  elapsed_ms: p.elapsed_ms,
                }
              : undefined;
          addLog(
            p.topic,
            p.message,
            p.error ? "error" : p.done ? "success" : "info",
            streaming
          );
        }
      );

      addLog("", `Starting generation for ${validTopics.length} topic(s)…`);

      const res = await generateNewsletter(
        validTopics.map((t) => ({ query: t.query })),
        sources,
        maxPapers,
        daysBack
      );
      setResults(res);
      const errors = res.filter((r) => r.error);
      if (errors.length === 0) {
        addLog(
          "",
          `All ${res.length} newsletter(s) generated successfully.`,
          "success"
        );
      } else {
        addLog(
          "",
          `${res.length - errors.length} succeeded, ${errors.length} failed.`,
          errors.length === res.length ? "error" : "info"
        );
      }
    } catch (e) {
      addLog("", `Fatal error: ${String(e)}`, "error");
    } finally {
      unlisten?.();
      setRunning(false);
    }
  }, [topics, sources, maxPapers, daysBack, config, addLog]);

  const value: GenerationState = {
    topics,
    sources,
    maxPapers,
    daysBack,
    running,
    log,
    results,
    showOptions,
    config,
    addTopic,
    updateTopic,
    removeTopic,
    toggleSource,
    handleGenerate,
    setMaxPapers,
    setDaysBack,
    setShowOptions,
  };

  return (
    <GenerationContext.Provider value={value}>
      {children}
    </GenerationContext.Provider>
  );
}

export function useGeneration(): GenerationState {
  const ctx = useContext(GenerationContext);
  if (!ctx) {
    throw new Error("useGeneration must be used within a GenerationProvider");
  }
  return ctx;
}
