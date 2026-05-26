import { useEffect, useState } from "react";

export type Theme = "paper" | "forest";

const STORAGE_KEY = "arboretum-theme";

function applyTheme(theme: Theme) {
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("data-theme", theme);
  }
}

function readInitialTheme(): Theme {
  if (typeof window === "undefined") return "paper";
  const saved = window.localStorage.getItem(STORAGE_KEY);
  return saved === "forest" ? "forest" : "paper";
}

let currentTheme: Theme = readInitialTheme();
const listeners = new Set<(t: Theme) => void>();

applyTheme(currentTheme);

export function setTheme(theme: Theme) {
  if (currentTheme === theme) return;
  currentTheme = theme;
  try {
    window.localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    /* noop */
  }
  applyTheme(theme);
  listeners.forEach((l) => l(theme));
}

export function useTheme(): [Theme, (t: Theme) => void] {
  const [theme, setLocal] = useState<Theme>(currentTheme);
  useEffect(() => {
    const listener = (t: Theme) => setLocal(t);
    listeners.add(listener);
    applyTheme(currentTheme);
    return () => {
      listeners.delete(listener);
    };
  }, []);
  return [theme, setTheme];
}
