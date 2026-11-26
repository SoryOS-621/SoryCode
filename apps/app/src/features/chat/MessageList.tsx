import { Icon, Markdown, Spinner } from "@sorycode/ui";
import { ToolActivityList } from "./ToolCallCard";
import type { ChatMessage, ToolActivity } from "./useChat";

function AssistantMessage({ message }: { message: ChatMessage }) {
  return (
    <div className="sory-msg sory-msg-assistant">
      <span className="sory-msg-avatar" aria-hidden="true">
        <Icon name="sparkles" size={15} />
      </span>
      <div className="sory-msg-body">
        <Markdown content={message.content} />
        {message.error ? <p className="sory-msg-error">This reply could not be completed.</p> : null}
        {message.streaming ? (
          <span className="sory-msg-cursor" aria-label="Writing">
            <Spinner size={12} />
          </span>
        ) : null}
      </div>
    </div>
  );
}

function UserMessage({ message }: { message: ChatMessage }) {
  return (
    <div className="sory-msg sory-msg-user">
      <div className={message.error ? "sory-bubble sory-bubble-error" : "sory-bubble"}>
        {message.content}
      </div>
    </div>
  );
}

export function MessageList({
  messages,
  tools,
}: {
  messages: ChatMessage[];
  tools: ToolActivity[];
}) {
  return (
    <div className="sory-messages">
      {messages.map((message) =>
        message.role === "user" ? (
          <UserMessage key={message.id} message={message} />
        ) : (
          <AssistantMessage key={message.id} message={message} />
        ),
      )}
      <ToolActivityList tools={tools} />
    </div>
  );
}
