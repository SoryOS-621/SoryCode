/** Shared domain types for every SoryCode client. */

export type TaskStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export type MessageRole = "system" | "user" | "assistant" | "tool";

export interface ProviderInfo {
  id: string;
  name: string;
  description: string;
  configured: boolean;
  freeOnly: boolean;
  defaultModel: string;
  models: string[];
}

export interface Selection {
  provider: string;
  model: string;
  models: string[];
}

export interface WorkspaceInfo {
  path: string;
  name: string;
  isGit: boolean;
  gitBranch: string | null;
}

export interface SessionSummary {
  id: string;
  projectId: string | null;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectSummary {
  id: string;
  name: string;
  rootPath: string;
  createdAt: string;
  updatedAt: string;
}

export interface TaskInfo {
  id: string;
  sessionId: string;
  description: string;
  status: TaskStatus;
  createdAt: string;
  updatedAt: string;
}

export interface HistoryMessage {
  role: MessageRole;
  content: string;
  toolCallId: string | null;
  toolCalls: ToolCall[];
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: unknown;
}

export interface FileEntry {
  path: string;
  name: string;
  isDir: boolean;
}

export interface AppInfo {
  version: string;
  dataPath: string;
  credentialsPath: string;
}

export interface BootstrapPayload {
  providers: ProviderInfo[];
  selection: Selection;
  workspace: WorkspaceInfo | null;
  sessions: SessionSummary[];
}
