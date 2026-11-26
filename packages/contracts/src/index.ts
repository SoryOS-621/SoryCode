/**
 * Wire contracts between the desktop shell (Rust) and the application (React).
 *
 * These types mirror the `serde` representations of `sorycode-events` and the
 * Tauri IPC DTOs. Engine events keep snake_case field names because that is
 * what the Rust side emits; UI-facing DTOs are camelCase.
 */

export type {
  AppInfo,
  BootstrapPayload,
  FileEntry,
  HistoryMessage,
  MessageRole,
  ProjectSummary,
  ProviderInfo,
  Selection,
  SessionSummary,
  TaskInfo,
  TaskStatus,
  ToolCall,
  WorkspaceInfo,
} from "@sorycode/types";

import type { TaskStatus } from "@sorycode/types";

export interface TaskStartedEvent {
  type: "task_started";
  project_id: string | null;
  session_id: string;
  task_id: string;
}

export interface TaskProgressEvent {
  type: "task_progress";
  session_id: string;
  task_id: string;
  message: string;
}

export interface MessageDeltaEvent {
  type: "message_delta";
  session_id: string;
  task_id: string | null;
  delta: string;
}

export interface ToolStartedEvent {
  type: "tool_started";
  session_id: string;
  task_id: string;
  tool_call_id: string;
  tool_name: string;
}

export interface ToolOutputEvent {
  type: "tool_output";
  session_id: string;
  task_id: string;
  tool_call_id: string;
  chunk: string;
}

export interface ToolCompletedEvent {
  type: "tool_completed";
  session_id: string;
  task_id: string;
  tool_call_id: string;
  is_error: boolean;
  summary: string;
}

export interface ApprovalRequiredEvent {
  type: "approval_required";
  session_id: string;
  task_id: string;
  approval_id: string;
  action: string;
  reason: string;
}

/**
 * Payload of the desktop shell's dedicated `sorycode://approval` channel.
 *
 * The engine event above carries ids the UI cannot resolve on its own, so the
 * shell re-emits pending requests with a fresh id the frontend answers with
 * `respond_approval`.
 */
export interface ApprovalRequest {
  approvalId: string;
  action: string;
  reason: string;
}

export interface TaskCompletedEvent {
  type: "task_completed";
  session_id: string;
  task_id: string;
  result: string;
}

export interface TaskFailedEvent {
  type: "task_failed";
  session_id: string;
  task_id: string;
  error: string;
}

export interface TaskCancelledEvent {
  type: "task_cancelled";
  session_id: string;
  task_id: string;
}

export type EngineEvent =
  | TaskStartedEvent
  | TaskProgressEvent
  | MessageDeltaEvent
  | ToolStartedEvent
  | ToolOutputEvent
  | ToolCompletedEvent
  | ApprovalRequiredEvent
  | TaskCompletedEvent
  | TaskFailedEvent
  | TaskCancelledEvent;

export interface EventEnvelope {
  id: string;
  timestamp: string;
  event: EngineEvent;
}

export interface TaskSnapshot {
  id: string;
  status: TaskStatus;
}

/** Commands accepted by the Core protocol (kept for SDK/transport parity). */
export type CoreCommand =
  | { type: "create_project"; name: string; root_path: string }
  | { type: "open_project"; project_id: string }
  | { type: "create_session"; project_id: string | null; title: string }
  | { type: "send_message"; session_id: string; content: string }
  | { type: "cancel_task"; task_id: string }
  | { type: "get_session"; session_id: string }
  | { type: "get_session_history"; session_id: string }
  | { type: "clear_session_history"; session_id: string }
  | { type: "list_sessions"; project_id: string | null }
  | { type: "get_task_status"; task_id: string };

export type CoreResponse =
  | { type: "ok" }
  | { type: "project_created"; project_id: string }
  | { type: "session_created"; session_id: string }
  | { type: "task_started"; task_id: string }
  | { type: "error"; message: string };
