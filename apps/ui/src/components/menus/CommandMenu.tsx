import type { ReactNode } from "react";
import { Icon, type IconName } from "../../icons/Icon";
import { Kbd } from "../../primitives/Feedback";

export interface CommandItem {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  icon?: IconName;
  group?: string;
}

export interface CommandMenuProps {
  items: CommandItem[];
  activeIndex?: number;
  onSelect: (item: CommandItem) => void;
  emptyLabel?: string;
  className?: string;
  header?: ReactNode;
  maxHeight?: number;
}

/** Presentational command palette. Keyboard state is owned by the caller. */
export function CommandMenu({
  items,
  activeIndex = 0,
  onSelect,
  emptyLabel = "No matching command",
  className,
  header,
  maxHeight = 280,
}: CommandMenuProps) {
  let lastGroup: string | undefined;

  return (
    <div
      className={["sory-menu", className].filter(Boolean).join(" ")}
      role="listbox"
      style={{ maxHeight }}
    >
      {header ? <div className="sory-menu-group">{header}</div> : null}
      {items.length === 0 ? <div className="sory-menu-empty">{emptyLabel}</div> : null}
      {items.map((item, index) => {
        const showGroup = item.group && item.group !== lastGroup;
        lastGroup = item.group;
        return (
          <div key={item.id}>
            {showGroup ? <div className="sory-menu-group">{item.group}</div> : null}
            <button
              type="button"
              role="option"
              aria-selected={index === activeIndex}
              className={[
                "sory-menu-item",
                index === activeIndex ? "sory-menu-item-active" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onSelect(item)}
            >
              {item.icon ? <Icon name={item.icon} size={14} /> : null}
              <span className="sory-menu-text">
                <span className="sory-menu-label">{item.label}</span>
                {item.description ? (
                  <span className="sory-menu-description">{item.description}</span>
                ) : null}
              </span>
              {item.shortcut ? <Kbd>{item.shortcut}</Kbd> : null}
            </button>
          </div>
        );
      })}
    </div>
  );
}

export function filterCommands(items: CommandItem[], query: string): CommandItem[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return items;
  return items.filter((item) => {
    const haystack = `${item.id} ${item.label} ${item.description ?? ""}`.toLowerCase();
    return haystack.includes(needle);
  });
}
