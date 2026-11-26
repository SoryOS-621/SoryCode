import { useCallback, useEffect, useRef, useState } from "react";
import type { EngineEvent, EventEnvelope, HistoryMessage } from "@sorycode/contracts";
import { coreClient } from "../../services";

export interface ToolActivity {
  id: string;
  name: string;
  status: "running" | "done" | "error";
  summary: string;
  /** Raw streamed output of a running call (capped). */
  output: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  streaming?: boolean;
  error?: boolean;
  createdAt: string;
}

function fromHistory(message: HistoryMessage, index: number): ChatMessage {
  return {
    id: `history-${index}`,
    role: message.role === "tool" ? "system" : message.role,
    content: message.content,
    createdAt: new Date().toISOString(),
  };
}

export interface UseChatResult {
  messages: ChatMessage[];
  tools: ToolActivity[];
  activeTaskId: string | null;
  /** Latest `task_progress` line, cleared when the task ends. */
  progress: string | null;
  loading: boolean;
  error: string | null;
  send: (content: string) => Promise<void>;
  cancel: () => Promise<void>;
  clear: () => Promise<void>;
  reload: () => Promise<void>;
}

/** Longest tool output kept in state before it is trimmed from the front. */
const MAX_TOOL_OUTPUT = 4000;

function appendOutput(previous: string, chunk: string): string {
  const next = previous + chunk;
  return next.length > MAX_TOOL_OUTPUT ? next.slice(next.length - MAX_TOOL_OUTPUT) : next;
}

export function useChat(sessionId: string | null): UseChatResult {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [tools, setTools] = useState<ToolActivity[]>([]);
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [progress, setProgress] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const streamId = useRef<string>("stream-current");

  const reload = useCallback(async () => {
    if (!sessionId) {
      setMessages([]);
      setTools([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const history = await coreClient.getHistory(sessionId);
      setMessages(history.map(fromHistory));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (!sessionId) return;
    streamId.current = `stream-${sessionId}-${Date.now()}`;

    return coreClient.onEvent((envelope: EventEnvelope) => {
      const event: EngineEvent = envelope.event;
      const eventSession = "session_id" in event ? event.session_id : null;
      if (eventSession !== sessionId) return;

      switch (event.type) {
        case "task_started":
          setActiveTaskId(event.task_id);
          setProgress(null);
          setError(null);
          break;
        case "task_progress":
          setProgress(event.message);
          break;
        case "message_delta":
          setMessages((current) => {
            const index = current.findIndex((message) => message.id === streamId.current);
            if (index === -1) {
              return [
                ...current,
                {
                  id: streamId.current,
                  role: "assistant",
                  content: event.delta,
                  streaming: true,
                  createdAt: new Date().toISOString(),
                },
              ];
            }
            const next = [...current];
            next[index] = {
              ...next[index]!,
              content: next[index]!.content + event.delta,
            };
            return next;
          });
          break;
        case "tool_started":
          setTools((current) => [
            ...current,
            {
              id: event.tool_call_id,
              name: event.tool_name,
              status: "running",
              summary: "",
              output: "",
            },
          ]);
          break;
        case "tool_output":
          setTools((current) =>
            current.map((tool) =>
              tool.id === event.tool_call_id
                ? { ...tool, output: appendOutput(tool.output, event.chunk) }
                : tool,
            ),
          );
          break;
        case "tool_completed":
          setTools((current) =>
            current.map((tool) =>
              tool.id === event.tool_call_id
                ? {
                    ...tool,
                    status: event.is_error ? "error" : "done",
                    // Keep streamed output when the core sent no summary.
                    summary: event.summary || tool.output,
                  }
                : tool,
            ),
          );
          break;
        case "task_completed":
          setProgress(null);
          setMessages((current) => {
            const index = current.findIndex((message) => message.id === streamId.current);
            if (index === -1) {
              return [
                ...current,
                {
                  id: `result-${event.task_id}`,
                  role: "assistant",
                  content: event.result,
                  createdAt: new Date().toISOString(),
                },
              ];
            }
            const next = [...current];
            const existing = next[index]!;
            next[index] = {
              ...existing,
              content: existing.content || event.result,
              streaming: false,
            };
            return next;
          });
          setActiveTaskId(null);
          break;
        case "task_failed":
          setProgress(null);
          setError(event.error);
          setMessages((current) =>
            current.map((message) =>
              message.id === streamId.current
                ? { ...message, streaming: false, error: true }
                : message,
            ),
          );
          setActiveTaskId(null);
          break;
        case "task_cancelled":
          setProgress(null);
          setActiveTaskId(null);
          setMessages((current) =>
            current.map((message) =>
              message.id === streamId.current ? { ...message, streaming: false } : message,
            ),
          );
          break;
        default:
          break;
      }
    });
  }, [sessionId]);

  const send = useCallback(
    async (content: string) => {
      if (!sessionId) throw new Error("no active session");
      const trimmed = content.trim();
      if (!trimmed) return;
      setError(null);
      streamId.current = `stream-${sessionId}-${Date.now()}`;
      setMessages((current) => [
        ...current,
        {
          id: `user-${Date.now()}`,
          role: "user",
          content: trimmed,
          createdAt: new Date().toISOString(),
        },
      ]);
      try {
        const taskId = await coreClient.sendMessage(sessionId, trimmed);
        setActiveTaskId(taskId);
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    },
    [sessionId],
  );

  const cancel = useCallback(async () => {
    if (!activeTaskId) return;
    await coreClient.cancelTask(activeTaskId);
    setActiveTaskId(null);
  }, [activeTaskId]);

  const clear = useCallback(async () => {
    if (!sessionId) return;
    await coreClient.clearHistory(sessionId);
    setMessages([]);
    setTools([]);
    setProgress(null);
  }, [sessionId]);

  return {
    messages,
    tools,
    activeTaskId,
    progress,
    loading,
    error,
    send,
    cancel,
    clear,
    reload,
  };
}
