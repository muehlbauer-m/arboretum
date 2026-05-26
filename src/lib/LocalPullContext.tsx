import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelLocalPull,
  getActivePull,
  pullLocalModel,
} from "./api";
import type { LocalPullProgress } from "./types";

/**
 * App-wide state for the local-AI model pull.
 *
 * Lives at App level so a long-running pull (often 1-5 GB) survives the
 * user navigating away from Settings. The provider also installs the
 * single global listener for the `local-pull-progress` event channel —
 * before this existed, leaving Settings unsubscribed every listener and
 * the pull silently disappeared from the UI.
 */
export interface LocalPullState {
  activeModel: string | null;
  progress: LocalPullProgress | null;
  /** Last error from a pull, cleared when a new one starts. */
  error: string | null;
  startPull: (host: string, model: string) => Promise<void>;
  cancelPull: () => Promise<void>;
}

const Ctx = createContext<LocalPullState | null>(null);

export function LocalPullProvider({ children }: { children: ReactNode }) {
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [progress, setProgress] = useState<LocalPullProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  // We need this in callbacks without re-binding to state.
  const activeModelRef = useRef<string | null>(null);
  activeModelRef.current = activeModel;

  // Single global listener — registered once for the lifetime of the app.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<LocalPullProgress>("local-pull-progress", (e) => {
      setProgress(e.payload);
      // Cancellation acks come through the same channel.
      if (e.payload.status === "cancelled") {
        setActiveModel(null);
      }
    }).then((u) => {
      unlisten = u;
    });
    return () => unlisten?.();
  }, []);

  // On mount, recover state if a pull was already running (e.g. after a
  // hot reload during dev; the Rust side outlives the WebView).
  useEffect(() => {
    getActivePull()
      .then((info) => {
        if (info.model) {
          setActiveModel(info.model);
        }
      })
      .catch(() => undefined);
  }, []);

  const startPull = useCallback(async (host: string, model: string) => {
    setError(null);
    setActiveModel(model);
    setProgress({
      model,
      status: "starting",
      total: null,
      completed: null,
    });
    try {
      await pullLocalModel(host, model);
      // Only clear active model if we still own it (protects against a
      // newer pull stomping the state on a fast double-click).
      if (activeModelRef.current === model) {
        setActiveModel(null);
        setProgress(null);
      }
    } catch (e) {
      const msg = String(e);
      // Cancellation by the user is not an error worth surfacing.
      if (!/cancel/i.test(msg)) {
        setError(msg);
      }
      if (activeModelRef.current === model) {
        setActiveModel(null);
        setProgress(null);
      }
    }
  }, []);

  const cancelPull = useCallback(async () => {
    try {
      await cancelLocalPull();
    } catch {
      // Best-effort — if the daemon is gone, just clear local state.
    }
    setActiveModel(null);
    setProgress(null);
  }, []);

  const value: LocalPullState = {
    activeModel,
    progress,
    error,
    startPull,
    cancelPull,
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useLocalPull(): LocalPullState {
  const ctx = useContext(Ctx);
  if (!ctx) {
    throw new Error("useLocalPull must be used within a LocalPullProvider");
  }
  return ctx;
}
