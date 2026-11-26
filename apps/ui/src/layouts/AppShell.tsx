import type { ReactNode } from "react";

export interface AppShellProps {
  sidebar?: ReactNode;
  children: ReactNode;
}

export function AppShell({ sidebar, children }: AppShellProps) {
  return (
    <div className="sory-app-shell">
      {sidebar ? <aside className="sory-sidebar">{sidebar}</aside> : null}
      <main className="sory-main">{children}</main>
    </div>
  );
}

export interface SidebarProps {
  header?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}

export function Sidebar({ header, children, footer }: SidebarProps) {
  return (
    <>
      {header ? <div className="sory-sidebar-header">{header}</div> : null}
      <div className="sory-sidebar-body">{children}</div>
      {footer ? <div className="sory-sidebar-footer">{footer}</div> : null}
    </>
  );
}

export interface AppHeaderProps {
  left?: ReactNode;
  right?: ReactNode;
}

export function AppHeader({ left, right }: AppHeaderProps) {
  return (
    <header className="sory-header">
      <div className="sory-header-left">{left}</div>
      <div className="sory-header-right">{right}</div>
    </header>
  );
}

export function AppContent({ children }: { children: ReactNode }) {
  return <div className="sory-content">{children}</div>;
}

export function SplitLayout({ left, right }: { left: ReactNode; right: ReactNode }) {
  return (
    <div className="sory-split">
      <div className="sory-split-pane">{left}</div>
      <div className="sory-split-pane">{right}</div>
    </div>
  );
}
