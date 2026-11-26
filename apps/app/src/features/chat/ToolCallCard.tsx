import { useState, type ReactNode } from "react";
import { Icon, Spinner, type IconName } from "@sorycode/ui";
import type { ToolActivity } from "./useChat";

/** tool name -> friendly title + icon family */
const TOOL_META: Record<string, { title: string; icon: IconName }> = {
  read_file: { title: "Read file", icon: "file" },
  write_file: { title: "Write file", icon: "file" },
  edit_file: { title: "Edit file", icon: "code" },
  apply_patch: { title: "Apply patch", icon: "code" },
  list_files: { title: "List files", icon: "folder" },
  search: { title: "Search code", icon: "search" },
  terminal: { title: "Run command", icon: "terminal" },
  run_tests: { title: "Run tests", icon: "play" },
  git_status: { title: "Git status", icon: "git" },
  git_diff: { title: "Git diff", icon: "git" },
  update_plan: { title: "Update plan", icon: "work" },
  tool_search: { title: "Find tools", icon: "search" },
  web_search: { title: "Web search", icon: "external" },
  github_search: { title: "GitHub search", icon: "git" },
};

function meta(name: string) {
  return TOOL_META[name] ?? { title: name.replace(/_/g, " "), icon: "wrench" as IconName };
}

/** Short target hint extracted from the summary (first line, truncated). */
function target(summary: string): string {
  const line = summary.split("\n")[0] ?? "";
  return line.length > 80 ? `${line.slice(0, 77)}…` : line;
}

function StatusGlyph({ status }: { status: ToolActivity["status"] }) {
  if (status === "running") return <Spinner size={12} />;
  if (status === "error") return <Icon name="alert" size={13} color="var(--sory-danger)" />;
  return <Icon name="check" size={13} color="var(--sory-success)" />;
}

export function ToolCallCard({ tool }: { tool: ToolActivity }) {
  const [open, setOpen] = useState(tool.status === "running");
  const { title, icon } = meta(tool.name);
  const body = tool.summary || tool.output;
  const detail = target(body);

  const stateClass =
    tool.status === "error"
      ? "sory-tool sory-tool-error"
      : tool.status === "running"
        ? "sory-tool sory-tool-running"
        : "sory-tool";

  return (
    <div className={stateClass}>
      <button type="button" className="sory-tool-head" onClick={() => setOpen((v) => !v)}>
        <Icon name={icon} size={14} color="var(--sory-text-muted)" />
        <span className="sory-tool-name">{title}</span>
        <span className="sory-tool-meta">
          {tool.name}
          {detail ? ` · ${detail}` : ""}
        </span>
        <StatusGlyph status={tool.status} />
        <Icon
          name={open ? "chevron-down" : "chevron-right"}
          size={13}
          color="var(--sory-text-subtle)"
        />
      </button>
      {open && body ? (
        <div
          className={tool.status === "error" ? "sory-tool-detail sory-tool-detail-error" : "sory-tool-detail"}
        >
          {body}
        </div>
      ) : null}
      {open && !body && tool.status === "running" ? (
        <div className="sory-tool-pending">Running…</div>
      ) : null}
    </div>
  );
}

/**
 * Renders the tool activity stream: running calls stay visible; completed
 * calls collapse behind a single "N tool calls" group (ChatGPT-style).
 */
export function ToolActivityList({ tools }: { tools: ToolActivity[] }) {
  const [groupOpen, setGroupOpen] = useState(false);
  if (tools.length === 0) return null;

  const running = tools.filter((tool) => tool.status === "running");
  const finished = tools.filter((tool) => tool.status !== "running");
  const errors = finished.filter((tool) => tool.status === "error").length;

  let groupLabel: ReactNode = null;
  if (finished.length > 0) {
    groupLabel = (
      <button type="button" className="sory-tool-toggle" onClick={() => setGroupOpen((v) => !v)}>
        <Icon name="wrench" size={11} />
        {finished.length} tool call{finished.length > 1 ? "s" : ""}
        {errors > 0 ? ` · ${errors} failed` : ""}
        <Icon
          name={groupOpen ? "chevron-down" : "chevron-right"}
          size={11}
          color="var(--sory-text-subtle)"
        />
      </button>
    );
  }

  return (
    <div className="sory-tools">
      {running.map((tool) => (
        <ToolCallCard key={tool.id} tool={tool} />
      ))}
      {groupLabel}
      {groupOpen ? finished.map((tool) => <ToolCallCard key={tool.id} tool={tool} />) : null}
    </div>
  );
}
