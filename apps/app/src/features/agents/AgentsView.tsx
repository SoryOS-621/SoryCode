import { Badge, EmptyState, Icon, Panel, PanelBody, PanelHeader } from "@sorycode/ui";
import type { IconName } from "@sorycode/ui";
import { useEngine } from "../../app/providers/EngineProvider";
import { useSelectedSessionId } from "../../stores/sessionStore";
import { useTasks } from "../work/useTasks";

const AGENTS: { id: string; label: string; icon: IconName; description: string }[] = [
  { id: "main", label: "Main agent", icon: "sparkles", description: "Plans the task and coordinates tools." },
  { id: "planner", label: "Planner", icon: "work", description: "Breaks complex requests into steps." },
  { id: "researcher", label: "Researcher", icon: "search", description: "Explores the codebase and gathers context." },
  { id: "coder", label: "Coder", icon: "code", description: "Implements changes precisely." },
  { id: "tester", label: "Tester", icon: "terminal", description: "Runs tests and reports failures." },
  { id: "reviewer", label: "Reviewer", icon: "check", description: "Reviews changes for correctness." },
  { id: "fixer", label: "Fixer", icon: "wrench", description: "Repairs failing tests and regressions." },
];

export function AgentsView() {
  const { sessions } = useEngine();
  const sessionId = useSelectedSessionId();
  const { tasks } = useTasks(sessionId);
  const session = sessions.find((item) => item.id === sessionId);
  const running = tasks.some((task) => task.status === "running");

  return (
    <div className="sory-view">
      <div className="sory-view-header">
        <div className="sory-view-heading">
          <h1 className="sory-view-title">Agents</h1>
          <p className="sory-view-subtitle">
            The orchestrator selects a role for each phase of a task. Roles are defined by the
            Rust core.
          </p>
        </div>
      </div>

      <Panel className="sory-fill">
        <PanelHeader
          title="Agent roster"
          description={session ? `Session: ${session.title}` : "No session selected"}
          actions={running ? <Badge tone="info">main agent running</Badge> : <Badge>idle</Badge>}
        />
        <PanelBody>
          <div className="sory-grid sory-grid-agents">
            {AGENTS.map((agent) => {
              const active = agent.id === "main" && running;
              return (
                <div key={agent.id} className={active ? "sory-card sory-card-active" : "sory-card"}>
                  <div className="sory-card-head">
                    <span className="sory-identity-icon sory-identity-icon-sm">
                      <Icon name={agent.icon} size={15} />
                    </span>
                    <strong className="sory-card-title">{agent.label}</strong>
                    <Badge tone={active ? "accent" : "default"}>
                      {active ? "active" : "ready"}
                    </Badge>
                  </div>
                  <p className="sory-card-text">{agent.description}</p>
                </div>
              );
            })}
          </div>
          {!sessionId ? (
            <EmptyState
              icon="agents"
              title="Select a session"
              description="Agent activity is scoped to the active session."
            />
          ) : null}
        </PanelBody>
      </Panel>
    </div>
  );
}
