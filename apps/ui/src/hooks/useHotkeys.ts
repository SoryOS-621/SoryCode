import { useEffect, useRef } from "react";

/** Bind keyboard shortcuts such as `mod+k`, `escape`, `mod+shift+p`. */
export function useHotkeys(bindings: Record<string, (event: KeyboardEvent) => void>) {
  const bindingsRef = useRef(bindings);
  bindingsRef.current = bindings;

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const parts: string[] = [];
      if (event.metaKey || event.ctrlKey) parts.push("mod");
      if (event.shiftKey) parts.push("shift");
      if (event.altKey) parts.push("alt");
      parts.push(event.key.toLowerCase());
      const combo = parts.join("+");
      const handler = bindingsRef.current[combo];
      if (handler) {
        event.preventDefault();
        handler(event);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
