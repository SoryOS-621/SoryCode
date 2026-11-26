import type { IconName } from "@sorycode/ui";

export type RouteId =
  | "chat"
  | "code"
  | "work"
  | "projects"
  | "workspaces"
  | "sessions"
  | "agents"
  | "settings";

export interface RouteDefinition {
  id: RouteId;
  label: string;
  icon: IconName;
  group: "Workspace" | "Manage" | "System";
  title: string;
  description: string;
}

export const ROUTES: RouteDefinition[] = [
  {
    id: "chat",
    label: "Chat",
    icon: "chat",
    group: "Workspace",
    title: "Chat",
    description: "Talk to the SoryCode agent",
  },
  {
    id: "code",
    label: "Code",
    icon: "code",
    group: "Workspace",
    title: "Code",
    description: "Browse the workspace",
  },
  {
    id: "work",
    label: "Work",
    icon: "work",
    group: "Workspace",
    title: "Work",
    description: "Long-running tasks and results",
  },
  {
    id: "projects",
    label: "Projects",
    icon: "projects",
    group: "Manage",
    title: "Projects",
    description: "Registered repositories",
  },
  {
    id: "workspaces",
    label: "Workspaces",
    icon: "workspaces",
    group: "Manage",
    title: "Workspaces",
    description: "Directories the agent can access",
  },
  {
    id: "sessions",
    label: "Sessions",
    icon: "sessions",
    group: "Manage",
    title: "Sessions",
    description: "Conversation history",
  },
  {
    id: "agents",
    label: "Agents",
    icon: "agents",
    group: "Manage",
    title: "Agents",
    description: "Orchestrator roles",
  },
  {
    id: "settings",
    label: "Settings",
    icon: "settings",
    group: "System",
    title: "Settings",
    description: "Providers, models and appearance",
  },
];

export function routeById(id: RouteId): RouteDefinition {
  return ROUTES.find((route) => route.id === id) ?? ROUTES[0]!;
}
