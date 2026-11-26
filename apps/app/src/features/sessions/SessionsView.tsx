import { useMemo, useState } from "react";
import {
  Badge,
  Button,
  EmptyState,
  Icon,
  Panel,
  PanelBody,
  PanelHeader,
  SearchInput,
} from "@sorycode/ui";
import { useEngine } from "../../app/providers/EngineProvider";
import { coreClient } from "../../services";
import { selectSession, useSelectedSessionId } from "../../stores/sessionStore";

export interface SessionsViewProps {
  onOpenChat: () => void;
}

export function SessionsView({ onOpenChat }: SessionsViewProps) {
  const { sessions, createSession, refreshSessions } = useEngine();
  const selected = useSelectedSessionId();
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return sessions;
    return sessions.filter((session) => session.title.toLowerCase().includes(needle));
  }, [sessions, query]);

  const open = (sessionId: string) => {
    selectSession(sessionId);
    onOpenChat();
  };

  return (
    <div className="sory-view">
      <div className="sory-view-header">
        <div className="sory-view-heading">
          <h1 className="sory-view-title">Sessions</h1>
          <p className="sory-view-subtitle">
            Every conversation is persisted by the Core storage.
          </p>
        </div>
        <div className="sory-slot-search">
          <SearchInput
            placeholder="Search sessions…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <Button
          size="sm"
          variant="primary"
          icon="plus"
          onClick={() => {
            void createSession().then(() => onOpenChat());
          }}
        >
          New session
        </Button>
      </div>

      <Panel className="sory-fill">
        <PanelHeader
          title="All sessions"
          description={`${sessions.length} session(s)`}
          actions={
            <Button size="sm" variant="ghost" icon="refresh" onClick={() => void refreshSessions()}>
              Refresh
            </Button>
          }
        />
        <PanelBody className="sory-panel-body-compact">
          {filtered.length === 0 ? (
            <EmptyState
              icon="sessions"
              title="No sessions"
              description="Start a session to see it listed here."
              action={
                <Button
                  variant="primary"
                  icon="plus"
                  onClick={() => void createSession().then(() => onOpenChat())}
                >
                  New session
                </Button>
              }
            />
          ) : (
            <div className="sory-list">
              {filtered.map((session) => (
                <div
                  key={session.id}
                  className={session.id === selected ? "sory-row sory-row-active" : "sory-row"}
                >
                  <Icon name="sessions" size={15} />
                  <div className="sory-row-main">
                    <div className="sory-row-title">{session.title}</div>
                    <div className="sory-row-meta">
                      {new Date(session.updatedAt).toLocaleString()} · {session.id.slice(0, 8)}
                    </div>
                  </div>
                  {session.id === selected ? <Badge tone="accent">active</Badge> : null}
                  <Button size="sm" variant="secondary" onClick={() => open(session.id)}>
                    Open
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    icon="trash"
                    onClick={() => {
                      void coreClient.clearHistory(session.id).then(refreshSessions);
                    }}
                  >
                    Clear
                  </Button>
                </div>
              ))}
            </div>
          )}
        </PanelBody>
      </Panel>
    </div>
  );
}
