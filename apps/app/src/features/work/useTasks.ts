import { useCallback, useEffect, useState } from "react";
import type { TaskInfo } from "@sorycode/contracts";
import { coreClient } from "../../services";

export function useTasks(sessionId: string | null) {
  const [tasks, setTasks] = useState<TaskInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!sessionId) {
      setTasks([]);
      return;
    }
    setLoading(true);
    try {
      setTasks(await coreClient.listTasks(sessionId));
      setError(null);
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
    return coreClient.onEvent((envelope) => {
      const event = envelope.event;
      if (!("session_id" in event) || event.session_id !== sessionId) return;
      if (
        event.type === "task_started" ||
        event.type === "task_completed" ||
        event.type === "task_failed" ||
        event.type === "task_cancelled"
      ) {
        void reload();
      }
    });
  }, [sessionId, reload]);

  return { tasks, loading, error, reload };
}
