import { useEffect, useId, useRef, useState } from "react";
import { useDocumentTheme } from "../../hooks/useDocumentTheme";
import { Icon } from "../../icons/Icon";

type RenderState = "rendering" | "ready" | "failed";

/** Mermaid is only pulled in when a diagram is actually present. */
let mermaidModule: Promise<typeof import("mermaid")> | null = null;

function loadMermaid(): Promise<typeof import("mermaid")> {
  mermaidModule ??= import("mermaid");
  return mermaidModule;
}

/** Mermaid can leave its scratch element behind when parsing fails. */
function cleanupScratch(id: string): void {
  for (const leftover of [`d${id}`, id]) {
    document.getElementById(leftover)?.remove();
  }
}

export interface MermaidBlockProps {
  /** Raw mermaid source, without the ```fence. */
  chart: string;
}

/**
 * Renders a fenced ```mermaid block as a real diagram.
 *
 * Falls back to a readable code block when the chart does not parse, so a
 * malformed diagram never breaks the surrounding message.
 */
export function MermaidBlock({ chart }: MermaidBlockProps) {
  const host = useRef<HTMLDivElement>(null);
  const rawId = useId();
  const theme = useDocumentTheme();
  const [state, setState] = useState<RenderState>("rendering");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!host.current) return;
    let cancelled = false;
    const id = `sorymermaid${rawId.replace(/[^a-zA-Z0-9]/g, "")}`;

    setState("rendering");
    setError(null);

    void (async () => {
      try {
        const { default: mermaid } = await loadMermaid();
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: theme === "dark" ? "dark" : "default",
          fontFamily: '"Inter", "Segoe UI", system-ui, sans-serif',
          flowchart: { htmlLabels: false, curve: "basis" },
        });
        const { svg } = await mermaid.render(id, chart);
        if (cancelled || !host.current) return;
        host.current.innerHTML = svg;
        setState("ready");
      } catch (cause) {
        cleanupScratch(id);
        if (cancelled) return;
        setError(cause instanceof Error ? cause.message : String(cause));
        setState("failed");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [chart, theme, rawId]);

  if (state === "failed") {
    return (
      <figure className="sory-mermaid sory-mermaid-failed">
        <figcaption className="sory-mermaid-caption">
          <Icon name="alert" size={13} />
          <span>Diagram could not be drawn</span>
        </figcaption>
        <pre className="sory-code">{chart}</pre>
        {error ? <span className="sory-mermaid-error">{error}</span> : null}
      </figure>
    );
  }

  return (
    <figure className="sory-mermaid">
      <figcaption className="sory-mermaid-caption">
        <Icon name="work" size={13} />
        <span>Diagram</span>
        {state === "rendering" ? <span className="sory-mermaid-status">drawing…</span> : null}
      </figcaption>
      <div className="sory-mermaid-canvas">
        {/* The host never receives React children: the SVG is injected
            imperatively, so React must not reconcile its subtree. */}
        <div ref={host} className="sory-mermaid-svg" data-state={state} aria-busy={state === "rendering"} />
        {state === "rendering" ? (
          <div className="sory-mermaid-skeleton" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
        ) : null}
      </div>
    </figure>
  );
}
