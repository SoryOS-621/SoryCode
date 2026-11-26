import type { HTMLAttributes, ReactNode } from "react";

export interface SurfaceProps extends HTMLAttributes<HTMLDivElement> {
  padding?: 0 | 1 | 2 | 3 | 4 | 5 | 6;
  children?: ReactNode;
}

export function Surface({ padding = 0, className, style, children, ...props }: SurfaceProps) {
  return (
    <div
      className={["sory-surface", className].filter(Boolean).join(" ")}
      style={{ padding: padding ? `var(--sory-space-${padding})` : undefined, ...style }}
      {...props}
    >
      {children}
    </div>
  );
}
