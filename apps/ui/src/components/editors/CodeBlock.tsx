import { useState } from "react";
import { IconButton } from "../buttons/IconButton";

export interface CodeBlockProps {
  code: string;
  language?: string;
  maxHeight?: number;
}

/** Fenced code panel with a language label and a copy action. */
export function CodeBlock({ code, language, maxHeight = 360 }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="sory-code-block">
      <div className="sory-code-block-bar">
        <span className="sory-code-block-lang">{language ?? "text"}</span>
        <IconButton
          icon={copied ? "check" : "copy"}
          label={copied ? "Copied" : "Copy code"}
          size={13}
          onClick={() => {
            void navigator.clipboard?.writeText(code);
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1200);
          }}
        />
      </div>
      <pre className="sory-code" style={{ maxHeight }}>
        {code}
      </pre>
    </div>
  );
}

export interface DiffViewProps {
  diff: string;
  maxHeight?: number;
}

export function DiffView({ diff, maxHeight = 360 }: DiffViewProps) {
  const lines = diff.split("\n");
  return (
    <pre className="sory-diff" style={{ maxHeight }}>
      {lines.map((line, index) => {
        const kind = line.startsWith("+") && !line.startsWith("+++")
          ? "add"
          : line.startsWith("-") && !line.startsWith("---")
            ? "remove"
            : "context";
        return (
          <div
            key={index}
            className={[
              "sory-diff-line",
              kind === "add" ? "sory-diff-add" : "",
              kind === "remove" ? "sory-diff-remove" : "",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            {line || " "}
          </div>
        );
      })}
    </pre>
  );
}
