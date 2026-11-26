import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { Spinner } from "../../primitives/Feedback";
import { Icon, type IconName } from "../../icons/Icon";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  icon?: IconName;
  children?: ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = "secondary",
    size = "md",
    loading = false,
    icon,
    className,
    disabled,
    children,
    ...props
  },
  ref,
) {
  return (
    <button
      ref={ref}
      className={[
        "sory-button",
        `sory-button-${variant}`,
        `sory-button-${size}`,
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? <Spinner size={14} /> : icon ? <Icon name={icon} size={14} /> : null}
      {children}
    </button>
  );
});
