import { isValidElement, memo, type ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { CodeBlock } from "./CodeBlock";
import { MermaidBlock } from "./MermaidBlock";

/**
 * Pull the source and language out of the `<code>` element react-markdown
 * nests inside every fenced block.
 */
function readBlock(children: ReactNode): { code: string; language?: string } {
  const child = Array.isArray(children) ? children[0] : children;
  if (!isValidElement<{ className?: string; children?: ReactNode }>(child)) {
    return { code: String(children ?? "") };
  }
  const className = child.props.className ?? "";
  const language = /language-([\w+#.-]+)/.exec(className)?.[1];
  const raw = child.props.children;
  const code = typeof raw === "string" ? raw : String(raw ?? "");
  return { code: code.replace(/\n+$/, ""), language };
}

type HeadingProps = { children?: ReactNode };
type LinkProps = { children?: ReactNode; href?: string };
type CodeProps = { children?: ReactNode; className?: string };
type CheckboxProps = { checked?: boolean; disabled?: boolean };

function Heading({ level, children }: { level: number; children?: ReactNode }) {
  const Tag = `h${Math.min(level + 1, 6)}` as "h2" | "h3" | "h4" | "h5" | "h6";
  return <Tag className={`sory-md-h${Math.min(level + 1, 6)}`}>{children}</Tag>;
}

const components: Components = {
  // Fenced blocks: mermaid becomes a diagram, everything else a code panel.
  pre: ({ children }) => {
    const { code, language } = readBlock(children);
    if (language === "mermaid") return <MermaidBlock chart={code} />;
    return <CodeBlock code={code} language={language} />;
  },
  code: ({ className, children }: CodeProps) => (
    <code className={className ? `sory-inline-code ${className}` : "sory-inline-code"}>
      {children}
    </code>
  ),
  a: ({ href, children }: LinkProps) => (
    <a href={href} target="_blank" rel="noreferrer noopener">
      {children}
    </a>
  ),
  h1: ({ children }: HeadingProps) => <Heading level={1}>{children}</Heading>,
  h2: ({ children }: HeadingProps) => <Heading level={2}>{children}</Heading>,
  h3: ({ children }: HeadingProps) => <Heading level={3}>{children}</Heading>,
  h4: ({ children }: HeadingProps) => <Heading level={4}>{children}</Heading>,
  h5: ({ children }: HeadingProps) => <Heading level={5}>{children}</Heading>,
  h6: ({ children }: HeadingProps) => <Heading level={6}>{children}</Heading>,
  input: ({ checked, disabled }: CheckboxProps) => (
    <input type="checkbox" checked={checked} disabled={disabled} readOnly />
  ),
};

export interface MarkdownProps {
  content: string;
  className?: string;
}

/**
 * Renders model output as rich text: headings, lists, tables, quotes, code
 * panels and mermaid diagrams. Raw HTML stays inert (react-markdown does not
 * execute it), which keeps untrusted model output safe to display.
 */
export const Markdown = memo(function Markdown({ content, className }: MarkdownProps) {
  return (
    <div className={["sory-markdown", className].filter(Boolean).join(" ")}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </ReactMarkdown>
    </div>
  );
});
