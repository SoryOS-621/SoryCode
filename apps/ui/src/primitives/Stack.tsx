import type { CSSProperties, HTMLAttributes, ReactNode } from "react";

type Space = 1 | 2 | 3 | 4 | 5 | 6;

export interface StackProps extends HTMLAttributes<HTMLDivElement> {
  direction?: "horizontal" | "vertical";
  gap?: Space;
  align?: CSSProperties["alignItems"];
  justify?: CSSProperties["justifyContent"];
  wrap?: boolean;
  children?: ReactNode;
}

export function Stack({
  direction = "vertical",
  gap = 3,
  align,
  justify,
  wrap,
  className,
  style,
  children,
  ...props
}: StackProps) {
  return (
    <div
      className={[
        "sory-stack",
        direction === "vertical" ? "sory-stack-vertical" : "sory-stack-horizontal",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      style={{
        gap: `var(--sory-space-${gap})`,
        alignItems: align,
        justifyContent: justify,
        flexWrap: wrap ? "wrap" : undefined,
        ...style,
      }}
      {...props}
    >
      {children}
    </div>
  );
}
