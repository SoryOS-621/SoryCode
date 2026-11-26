import { useEffect, useRef } from "react";
import { Badge, Button, EmptyState, IconButton, Spinner } from "@sorycode/ui";
import { useEngine } from "../../app/providers/EngineProvider";
import { Composer } from "./Composer";
import { MessageList } from "./MessageList";
import { useChat } from "./useChat";

export interface ChatViewProps {
  sessionId: string | null;
  onNewSession: () => void;
  onOpenSettings: () => void;
  onHelp: () => void;
}

const SUGGESTIONS = [
  "Explain the architecture of this repository",
  "Find and fix the failing build",
  "Review the current diff for bugs",
];

export function ChatView({ sessionId, onNewSession, onOpenSettings, onHelp }: ChatViewProps) {
  const { selection, sessions, workspace } = useEngine();
  const chat = useChat(sessionId);
  const bottom = useRef<HTMLDivElement>(null);
  const session = sessions.find((item) => item.id === sessionId);

  useEffect(() => {
    bottom.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [chat.messages, chat.tools]);

  if (!sessionId) {
    return (
      <div className="sory-chat">
        <EmptyState
          icon="chat"
          title="No active session"
          description="Create a session to start working with the SoryCode agent."
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
    <div className="sory-chat">
      <div className="sory-chat-toolbar">
        <div className="sory-chat-toolbar-main">
          <div className="sory-chat-title">
            <strong>{session?.title ?? "Session"}</strong>
            {selection ? <Badge tone="accent">{selection.model}</Badge> : null}
            {selection ? <Badge>{selection.provider}</Badge> : null}
            {workspace?.isGit && workspace.gitBranch ? (
              <Badge tone="info">{workspace.gitBranch}</Badge>
            ) : null}
          </div>
          <div className="sory-chat-path">{workspace?.path}</div>
        </div>
        <IconButton icon="plus" label="New session" onClick={onNewSession} />
        <IconButton icon="trash" label="Clear conversation" onClick={() => void chat.clear()} />
        <IconButton icon="settings" label="Settings" onClick={onOpenSettings} />
      </div>

      <div className="sory-chat-scroll">
        <div className="sory-chat-column">
          {chat.loading ? (
            <div className="sory-loading">
              <Spinner />
            </div>
          ) : chat.messages.length === 0 ? (
            <div className="sory-chat-hero">
              <div className="sory-chat-hero-mark">S</div>
              <div>
                <div className="sory-chat-hero-title">How can I help you today?</div>
                <div className="sory-chat-hero-subtitle">
                  {workspace?.path ?? "No workspace"} · {selection?.provider ?? ""} /{" "}
                  {selection?.model ?? ""}
                </div>
              </div>
              <div className="sory-chat-suggestions">
                {SUGGESTIONS.map((suggestion) => (
                  <button
                    key={suggestion}
                    className="sory-chat-suggestion"
                    onClick={() => void chat.send(suggestion)}
                  >
                    {suggestion}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <MessageList messages={chat.messages} tools={chat.tools} />
          )}
          <div ref={bottom} />
        </div>
      </div>

      <div className="sory-chat-footer">
        {chat.progress && chat.activeTaskId ? (
          <div className="sory-chat-progress" role="status" aria-live="polite">
            <Spinner size={12} />
            <span>{chat.progress}</span>
          </div>
        ) : null}
        {chat.error ? <div className="sory-chat-error">{chat.error}</div> : null}

        <Composer
          onSend={chat.send}
          onNewSession={onNewSession}
          onClear={() => void chat.clear()}
          onOpenSettings={onOpenSettings}
          onHelp={onHelp}
          busy={chat.activeTaskId !== null}
          onCancel={() => void chat.cancel()}
        />
      </div>
    </div>
  );
}
