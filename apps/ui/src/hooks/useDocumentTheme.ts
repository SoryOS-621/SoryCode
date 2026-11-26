import { useSyncExternalStore } from "react";

/**
 * Subscribes to the theme actually applied on `<html data-theme>`.
 *
 * `ThemeProvider` owns the value, but components rendered outside its tree
 * (markdown diagrams, embedded previews) still need to react to a switch.
 * A single module-level observer keeps the subscription count at one.
 */
type ResolvedTheme = "light" | "dark";

const listeners = new Set<() => void>();

function readTheme(): ResolvedTheme {
  if (typeof document === "undefined") return "light";
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

let snapshot = readTheme();

if (typeof document !== "undefined" && typeof MutationObserver !== "undefined") {
  const observer = new MutationObserver(() => {
    const next = readTheme();
    if (next === snapshot) return;
    snapshot = next;
    for (const listener of listeners) listener();
  });
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Current resolved theme, updated live when the document theme changes. */
export function useDocumentTheme(): ResolvedTheme {
  return useSyncExternalStore(subscribe, readTheme, readTheme);
}
