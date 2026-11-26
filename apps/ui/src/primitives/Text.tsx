import type { ElementType, HTMLAttributes, ReactNode } from "react";

export type TextSize = "xs" | "sm" | "md" | "lg" | "xl" | "2xl";
export type TextTone = "default" | "muted" | "subtle" | "strong";

export interface TextProps extends HTMLAttributes<HTMLElement> {
  as?: ElementType;
  size?: TextSize;
  tone?: TextTone;
  mono?: boolean;
  truncate?: boolean;
  children?: ReactNode;
}

export function Text({
  as: Component = "span",
  size = "md",
  tone = "default",
  mono,
  truncate,
  className,
  style,
  children,
  ...props
}: TextProps) {
  return (
    <Component
      className={[
        "sory-text",
        `sory-text-${size}`,
        tone !== "default" ? `sory-text-${tone}` : "",
        mono ? "sory-text-mono" : "",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      style={{
        overflow: truncate ? "hidden" : undefined,
        textOverflow: truncate ? "ellipsis" : undefined,
        whiteSpace: truncate ? "nowrap" : undefined,
        ...style,
      }}
      {...props}
    >
      {children}
    </Component>
  );
}
