import { useEffect, useRef, useState, type ReactNode } from "react";
import { Icon, type IconName } from "../../icons/Icon";

export interface DropdownItem {
  id: string;
  label: string;
  icon?: IconName;
  danger?: boolean;
  disabled?: boolean;
}

export interface DropdownMenuProps {
  trigger: ReactNode;
  items: DropdownItem[];
  onSelect: (item: DropdownItem) => void;
  align?: "start" | "end";
  emptyLabel?: string;
}

export function DropdownMenu({
  trigger,
  items,
  onSelect,
  align = "start",
  emptyLabel = "No items",
}: DropdownMenuProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: MouseEvent) => {
      if (root.current && !root.current.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="sory-dropdown" ref={root}>
      <span onClick={() => setOpen((value) => !value)}>{trigger}</span>
      {open ? (
        <div
          className="sory-menu sory-dropdown-panel"
          style={align === "end" ? { right: 0 } : { left: 0 }}
          role="menu"
        >
          {items.length === 0 ? (
            <div className="sory-menu-empty">{emptyLabel}</div>
          ) : (
            items.map((item) => (
              <button
                key={item.id}
                type="button"
                role="menuitem"
                className="sory-menu-item"
                style={item.danger ? { color: "var(--sory-danger)" } : undefined}
                disabled={item.disabled}
                onClick={() => {
                  setOpen(false);
                  onSelect(item);
                }}
              >
                {item.icon ? <Icon name={item.icon} size={14} /> : null}
                <span>{item.label}</span>
              </button>
            ))
          )}
        </div>
      ) : null}
    </div>
  );
}
