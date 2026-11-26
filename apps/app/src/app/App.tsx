import {
  AppContent,
  AppHeader,
  AppShell,
  Badge,
  Button,
  Dialog,
  IconButton,
  Kbd,
  Sidebar,
  SidebarNav,
  StatusDot,
  useTheme,
  type SidebarNavItem,
} from "@sorycode/ui";
import { useEngine } from "./providers/EngineProvider";
import { ROUTES, routeById, type RouteId } from "../routes";
import { useRoute } from "../routes/useRoute";
import { selectSession, useSelectedSessionId } from "../stores/sessionStore";
import { setHelpOpen, useUiState } from "../stores/uiStore";
import { ChatPage } from "../pages/ChatPage";
import { CodePage } from "../pages/CodePage";
import { WorkPage } from "../pages/WorkPage";
import { ProjectsPage } from "../pages/ProjectsPage";
import { WorkspacesPage } from "../pages/WorkspacesPage";
import { SessionsPage } from "../pages/SessionsPage";
import { AgentsPage } from "../pages/AgentsPage";
import { SettingsPage } from "../pages/SettingsPage";
import { ApprovalDialog } from "../features/approvals/ApprovalDialog";

const GROUPS: Array<"Workspace" | "Manage" | "System"> = ["Workspace", "Manage", "System"];

const HELP_ITEMS = [
  { key: "/new", label: "Start a new session" },
  { key: "/clear", label: "Clear the current conversation" },
  { key: "/model", label: "Change the active model" },
  { key: "/provider", label: "Change the active provider" },
  { key: "/settings", label: "Open settings" },
  { key: "/help", label: "Show this help" },
];

export function App() {
  const [route, navigate] = useRoute();
  const { selection, workspace, sessions, createSession } = useEngine();
  const { resolved, toggle } = useTheme();
  const ui = useUiState();
  const selectedSessionId = useSelectedSessionId();
  const definition = routeById(route);

  const sections = GROUPS.map((group) => ({
    label: group,
    items: ROUTES.filter((item) => item.group === group).map<SidebarNavItem>((item) => ({
      id: item.id,
      label: item.label,
      icon: item.icon,
      badge:
        item.id === "sessions" && sessions.length > 0 ? (
          <Badge>{sessions.length}</Badge>
        ) : undefined,
    })),
  }));

  const recent = sessions.slice(0, 6);

  return (
    <>
      <AppShell
        sidebar={
          <Sidebar
            header={
              <>
                <span className="app-logo">S</span>
                <div className="app-brand">
                  <div className="app-brand-name">SoryCode</div>
                  <div className="app-brand-tagline">agentic coding engine</div>
                </div>
              </>
            }
            footer={
              <div className="app-sidebar-footer">
                <div className="app-sidebar-status">
                  <StatusDot tone={selection ? "success" : "warning"} label={selection?.provider} />
                  <span className="app-sidebar-status-label">
                    {selection ? `${selection.provider} / ${selection.model}` : "No provider"}
                  </span>
                  <IconButton
                    icon={resolved === "dark" ? "sun" : "moon"}
                    label="Toggle theme"
                    onClick={toggle}
                    size={14}
                  />
                </div>
                <Button
                  size="sm"
                  variant="primary"
                  icon="plus"
                  onClick={() => {
                    void createSession().then(() => navigate("chat"));
                  }}
                >
                  New session
                </Button>
              </div>
            }
          >
            <SidebarNav
              sections={sections}
              active={route}
              onSelect={(id) => navigate(id as RouteId)}
            />

            <div className="sory-nav-section">Recent sessions</div>
            <div className="app-recent">
              {recent.length === 0 ? (
                <div className="app-recent-empty">No sessions yet</div>
              ) : (
                recent.map((session) => (
                  <button
                    key={session.id}
                    type="button"
                    className={[
                      "app-recent-item",
                      session.id === selectedSessionId ? "app-recent-item-active" : "",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    onClick={() => {
                      selectSession(session.id);
                      navigate("chat");
                    }}
                  >
                    <span>{session.title}</span>
                  </button>
                ))
              )}
            </div>
          </Sidebar>
        }
      >
        <AppHeader
          left={
            <div className="app-header-heading">
              <div className="app-header-title">{definition.title}</div>
              <div className="app-header-description">{definition.description}</div>
            </div>
          }
          right={
            <>
              {workspace ? (
                <Badge tone="info">
                  {workspace.name}
                  {workspace.isGit && workspace.gitBranch ? ` · ${workspace.gitBranch}` : ""}
                </Badge>
              ) : null}
              <IconButton icon="settings" label="Settings" onClick={() => navigate("settings")} />
              <IconButton icon="alert" label="Help" onClick={() => setHelpOpen(true)} />
            </>
          }
        />
        <AppContent>
          <RouteView route={route} navigate={navigate} />
        </AppContent>
      </AppShell>

      <Dialog
        open={ui.helpOpen}
        onClose={() => setHelpOpen(false)}
        title="SoryCode help"
        footer={
          <Button variant="primary" onClick={() => setHelpOpen(false)}>
            Close
          </Button>
        }
      >
        <div className="sory-help-section">
          <div className="sory-help-title">Slash commands</div>
          <div className="sory-help-list">
            {HELP_ITEMS.map((item) => (
              <div key={item.key} className="sory-help-item">
                <Kbd>{item.key}</Kbd>
                <span>{item.label}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="sory-help-section">
          <div className="sory-help-title">Keyboard</div>
          <div className="sory-help-list">
            <div className="sory-help-item">
              <Kbd>Enter</Kbd>
              <span>Send message</span>
            </div>
            <div className="sory-help-item">
              <Kbd>Shift+Enter</Kbd>
              <span>New line</span>
            </div>
            <div className="sory-help-item">
              <Kbd>↑</Kbd>
              <Kbd>↓</Kbd>
              <span>Navigate the slash menu</span>
            </div>
          </div>
        </div>
      </Dialog>

      <ApprovalDialog />
    </>
  );
}

function RouteView({ route, navigate }: { route: RouteId; navigate: (route: RouteId) => void }) {
  switch (route) {
    case "chat":
      return <ChatPage onNavigate={navigate} />;
    case "code":
      return <CodePage />;
    case "work":
      return <WorkPage onNavigate={navigate} />;
    case "projects":
      return <ProjectsPage />;
    case "workspaces":
      return <WorkspacesPage />;
    case "sessions":
      return <SessionsPage onNavigate={navigate} />;
    case "agents":
      return <AgentsPage />;
    case "settings":
      return <SettingsPage />;
    default:
      return <ChatPage onNavigate={navigate} />;
  }
}
