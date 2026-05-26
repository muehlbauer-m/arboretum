import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { getConfig, saveConfig, scanConflicts } from "./api";
import type {
  CompetitionProfile,
  ConflictScanResult,
  LogEntry,
  ProgressEvent,
  AppConfig,
} from "./types";

interface ConflictState {
  profiles: CompetitionProfile[];
  results: ConflictScanResult[];
  scanning: boolean;
  log: LogEntry[];
  config: AppConfig | null;
  addProfile: () => void;
  updateProfile: (id: string, updates: Partial<CompetitionProfile>) => void;
  removeProfile: (id: string) => void;
  toggleProfile: (id: string) => void;
  handleScan: () => void;
}

const ConflictContext = createContext<ConflictState | null>(null);

export function ConflictProvider({ children }: { children: ReactNode }) {
  const [profiles, setProfiles] = useState<CompetitionProfile[]>([]);
  const [results, setResults] = useState<ConflictScanResult[]>([]);
  const [scanning, setScanning] = useState(false);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);

  // Load config on mount
  useEffect(() => {
    getConfig()
      .then((cfg) => {
        setConfig(cfg);
        setProfiles(cfg.conflict_profiles ?? []);
      })
      .catch(console.error);
  }, []);

  const addLog = useCallback(
    (topic: string, message: string, type: LogEntry["type"] = "info") => {
      setLog((prev) => [
        ...prev,
        {
          ts: new Date().toLocaleTimeString("en-US", {
            hour12: false,
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
          }),
          topic,
          message,
          type,
        },
      ]);
    },
    []
  );

  const addProfile = useCallback(() => {
    const newProfile: CompetitionProfile = {
      id: crypto.randomUUID(),
      name: "",
      research_description: "",
      key_terms: [],
      own_papers: [],
      enabled: true,
      last_scanned: null,
      last_overlap_count: 0,
    };
    setProfiles((prev) => [...prev, newProfile]);
  }, []);

  const updateProfile = useCallback(
    (id: string, updates: Partial<CompetitionProfile>) => {
      setProfiles((prev) =>
        prev.map((p) => (p.id === id ? { ...p, ...updates } : p))
      );
    },
    []
  );

  const removeProfile = useCallback((id: string) => {
    setProfiles((prev) => prev.filter((p) => p.id !== id));
  }, []);

  const toggleProfile = useCallback((id: string) => {
    setProfiles((prev) =>
      prev.map((p) => (p.id === id ? { ...p, enabled: !p.enabled } : p))
    );
  }, []);

  const handleScan = useCallback(async () => {
    if (!config) return;

    const enabledProfiles = profiles.filter((p) => p.enabled);
    if (enabledProfiles.length === 0) {
      addLog("", "No enabled profiles to scan.", "error");
      return;
    }

    // Save current config with profiles before scanning
    const updatedConfig: AppConfig = { ...config, conflict_profiles: profiles };
    try {
      await saveConfig(updatedConfig);
      setConfig(updatedConfig);
    } catch (e) {
      addLog("", `Failed to save config: ${String(e)}`, "error");
      return;
    }

    setScanning(true);
    setResults([]);
    setLog([]);

    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<ProgressEvent>(
        "conflict-progress",
        (event) => {
          const p = event.payload;
          addLog(
            p.topic,
            p.message,
            p.error ? "error" : p.done ? "success" : "info"
          );
        }
      );

      addLog(
        "",
        `Starting conflict scan for ${enabledProfiles.length} profile(s)…`
      );

      const sources = config.default_sources;
      const maxPapers = config.conflict_settings?.max_papers_per_source ?? 50;
      const daysBack = config.conflict_settings?.scan_days_back ?? 90;

      const res = await scanConflicts(
        enabledProfiles,
        sources,
        maxPapers,
        daysBack
      );
      setResults(res);

      const errors = res.filter((r) => r.error);
      const totalOverlaps = res.reduce(
        (sum, r) => sum + r.overlaps.length,
        0
      );

      if (errors.length === 0) {
        addLog(
          "",
          `Scan complete. Found ${totalOverlaps} overlap(s) across ${res.length} profile(s).`,
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
      setScanning(false);
    }
  }, [profiles, config, addLog]);

  const value: ConflictState = {
    profiles,
    results,
    scanning,
    log,
    config,
    addProfile,
    updateProfile,
    removeProfile,
    toggleProfile,
    handleScan,
  };

  return (
    <ConflictContext.Provider value={value}>
      {children}
    </ConflictContext.Provider>
  );
}

export function useConflict(): ConflictState {
  const ctx = useContext(ConflictContext);
  if (!ctx) {
    throw new Error("useConflict must be used within a ConflictProvider");
  }
  return ctx;
}
