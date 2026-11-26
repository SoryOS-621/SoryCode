import type { HTMLAttributes, ReactNode } from "react";

export interface DividerProps extends HTMLAttributes<HTMLHRElement> {}

export function Divider({ className, ...props }: DividerProps) {
  return <hr className={["sory-divider", className].filter(Boolean).join(" ")} {...props} />;
}

export interface SpinnerProps {
  size?: number;
  label?: string;
}

export function Spinner({ size = 16, label = "Loading" }: SpinnerProps) {
  return (
    <span
      className="sory-spinner"
      role="status"
      aria-label={label}
      style={{ width: size, height: size }}
    />
  );
}

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: "default" | "accent" | "success" | "warning" | "danger" | "info";
  children?: ReactNode;
}

export function Badge({ tone = "default", className, children, ...props }: BadgeProps) {
  return (
    <span
      className={["sory-badge", tone !== "default" ? `sory-badge-${tone}` : "", className]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {children}
    </span>
  );
}

export interface KbdProps extends HTMLAttributes<HTMLElement> {
  children?: ReactNode;
}

export function Kbd({ className, children, ...props }: KbdProps) {
  return (
    <kbd className={["sory-kbd", className].filter(Boolean).join(" ")} {...props}>
      {children}
    </kbd>
  );
}

export interface TooltipProps {
  label: string;
  children: ReactNode;
}

export function Tooltip({ label, children }: TooltipProps) {
  return (
    <span className="sory-tooltip">
      {children}
      <span className="sory-tooltip-content" role="tooltip">
        {label}
      </span>
    </span>
  );
}
