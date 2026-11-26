import type { ButtonHTMLAttributes } from "react";
import { Icon, type IconName } from "../../icons/Icon";
import { Tooltip } from "../../primitives/Feedback";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: IconName;
  label: string;
  size?: number;
  tooltip?: boolean;
}

export function IconButton({
  icon,
  label,
  size = 16,
  tooltip = true,
  className,
  ...props
}: IconButtonProps) {
  const button = (
    <button
      type="button"
      className={["sory-icon-button", className].filter(Boolean).join(" ")}
      aria-label={label}
      {...props}
    >
      <Icon name={icon} size={size} />
    </button>
  );
  return tooltip ? <Tooltip label={label}>{button}</Tooltip> : button;
}
