import type { CSSProperties, ReactNode } from "react";
import { Icon, type IconName } from "../../icons/Icon";

export interface PanelProps {
  className?: string;
  style?: CSSProperties;
  children: ReactNode;
}

export function Panel({ className, style, children }: PanelProps) {
  return (
    <section className={["sory-panel", className].filter(Boolean).join(" ")} style={style}>
      {children}
    </section>
  );
}

export interface PanelHeaderProps {
  title: string;
  actions?: ReactNode;
  description?: string;
}

export function PanelHeader({ title, description, actions }: PanelHeaderProps) {
  return (
    <header className="sory-panel-header">
      <div>
        <h2 className="sory-panel-title">{title}</h2>
        {description ? <p className="sory-panel-description">{description}</p> : null}
      </div>
      {actions}
    </header>
  );
}

export interface PanelBodyProps {
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
}

export function PanelBody({ children, className, style }: PanelBodyProps) {
  return (
    <div className={["sory-panel-body", className].filter(Boolean).join(" ")} style={style}>
      {children}
    </div>
  );
}

export interface EmptyStateProps {
  icon?: IconName;
  title: string;
  description?: string;
  action?: ReactNode;
}

export function EmptyState({ icon = "sparkles", title, description, action }: EmptyStateProps) {
  return (
    <div className="sory-empty">
      <span className="sory-empty-icon">
        <Icon name={icon} size={20} />
      </span>
      <strong className="sory-empty-title">{title}</strong>
      {description ? <span className="sory-empty-description">{description}</span> : null}
      {action}
    </div>
  );
}

export interface StatusDotProps {
  tone?: "default" | "success" | "warning" | "danger" | "accent";
  label?: string;
}

export function StatusDot({ tone = "default", label }: StatusDotProps) {
  return (
    <span
      className={["sory-status-dot", tone !== "default" ? `sory-status-dot-${tone}` : ""]
        .filter(Boolean)
        .join(" ")}
      title={label}
      aria-label={label}
    />
  );
}
