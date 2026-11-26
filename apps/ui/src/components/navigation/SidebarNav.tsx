import type { ReactNode } from "react";
import { Icon, type IconName } from "../../icons/Icon";

export interface SidebarNavItem {
  id: string;
  label: string;
  icon: IconName;
  badge?: ReactNode;
}

export interface SidebarNavProps {
  items?: SidebarNavItem[];
  active: string;
  onSelect: (id: string) => void;
  sections?: { label: string; items: SidebarNavItem[] }[];
}

export function SidebarNav({ items = [], active, onSelect, sections }: SidebarNavProps) {
  const groups = sections ?? [{ label: "", items }];
  return (
    <nav className="sory-nav" aria-label="Main navigation">
      {groups.map((group) => (
        <div key={group.label || "default"}>
          {group.label ? <div className="sory-nav-section">{group.label}</div> : null}
          {group.items.map((item) => (
            <button
              key={item.id}
              type="button"
              className={[
                "sory-nav-item",
                active === item.id ? "sory-nav-item-active" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => onSelect(item.id)}
            >
              <Icon name={item.icon} size={16} />
              <span className="sory-nav-label">{item.label}</span>
              {item.badge}
            </button>
          ))}
        </div>
      ))}
    </nav>
  );
}
