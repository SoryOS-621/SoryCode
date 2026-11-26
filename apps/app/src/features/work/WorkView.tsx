import {
  Badge,
  Button,
  EmptyState,
  Panel,
  PanelBody,
  PanelHeader,
  Spinner,
  StatusDot,
} from "@sorycode/ui";
import type { TaskStatus } from "@sorycode/types";
import { useEngine } from "../../app/providers/EngineProvider";
import { coreClient } from "../../services";
import { useTasks } from "./useTasks";

const STATUS_TONE: Record<TaskStatus, "default" | "accent" | "success" | "warning" | "danger" | "info"> = {
  pending: "default",
  running: "info",
  completed: "success",
  failed: "danger",
  cancelled: "warning",
};

const STATUS_DOT: Record<TaskStatus, "default" | "success" | "warning" | "danger" | "accent"> = {
  pending: "default",
  running: "accent",
  completed: "success",
  failed: "danger",
  cancelled: "warning",
};

export interface WorkViewProps {
  sessionId: string | null;
  onOpenChat: () => void;
  onNewSession: () => void;
}

export function WorkView({ sessionId, onOpenChat, onNewSession }: WorkViewProps) {
  const { sessions } = useEngine();
  const { tasks, loading, error, reload } = useTasks(sessionId);
  const session = sessions.find((item) => item.id === sessionId);

  if (!sessionId) {
    return (
      <div className="sory-view">
        <EmptyState
          icon="work"
          title="No session selected"
          description="Tasks are attached to a session. Start one to see work here."
          action={
            <Button variant="primary" icon="plus" onClick={onNewSession}>
              New session
            </Button>
          }
        />
      </div>
    );
  }

  return (
    <div className="sory-view">
      <div className="sory-view-header">
        <div className="sory-view-heading">
          <h1 className="sory-view-title">Work queue</h1>
          <p className="sory-view-subtitle">{session?.title}</p>
        </div>
        <Button size="sm" variant="secondary" icon="refresh" onClick={() => void reload()}>
          Refresh
        </Button>
        <Button size="sm" variant="primary" icon="chat" onClick={onOpenChat}>
          Open chat
        </Button>
      </div>

      {error ? <div className="sory-error">{error}</div> : null}

      <Panel className="sory-fill">
        <PanelHeader
          title="Tasks"
          description={`${tasks.length} task${tasks.length === 1 ? "" : "s"} in this session`}
        />
        <PanelBody className="sory-panel-body-compact">
          {loading && tasks.length === 0 ? (
            <div className="sory-loading">
              <Spinner />
            </div>
          ) : tasks.length === 0 ? (
            <EmptyState
              icon="work"
              title="No work yet"
              description="Send a request from the chat to create a task."
            />
          ) : (
            <div className="sory-list">
              {tasks.map((task) => (
                <div key={task.id} className="sory-row">
                  <StatusDot tone={STATUS_DOT[task.status]} label={task.status} />
                  <div className="sory-row-main">
                    <div className="sory-row-title">{task.description || task.id}</div>
                    <div className="sory-row-meta">
                      {new Date(task.createdAt).toLocaleString()}
                    </div>
                  </div>
                  <Badge tone={STATUS_TONE[task.status]}>{task.status}</Badge>
                  {task.status === "running" ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      icon="stop"
                      onClick={() => void coreClient.cancelTask(task.id).then(reload)}
                    >
                      Stop
                    </Button>
                  ) : null}
                </div>
              ))}
            </div>
          )}
        </PanelBody>
      </Panel>
    </div>
  );
}
