import type {
  AppInfo,
  ApprovalRequest,
  BootstrapPayload,
  EventEnvelope,
  FileEntry,
  HistoryMessage,
  ProjectSummary,
  ProviderInfo,
  Selection,
  SessionSummary,
  TaskInfo,
  WorkspaceInfo,
} from "@sorycode/contracts";

export type { ApprovalRequest };

/**
 * Contract used by the application to reach SoryCode Core.
 *
 * `TauriClient` is the production adapter; `MockClient` keeps the UI usable in
 * a plain browser during development. Features must never import Tauri APIs
 * directly.
 */
export interface CoreClient {
  bootstrap(): Promise<BootstrapPayload>;
  listProviders(): Promise<ProviderInfo[]>;
  getSelection(): Promise<Selection>;
  selectProvider(provider: string): Promise<Selection>;
  selectModel(model: string): Promise<Selection>;
  connectProvider(provider: string, apiKey: string): Promise<Selection>;

  createSession(title: string, projectId?: string | null): Promise<SessionSummary>;
  listSessions(projectId?: string | null): Promise<SessionSummary[]>;
  getHistory(sessionId: string): Promise<HistoryMessage[]>;
  clearHistory(sessionId: string): Promise<void>;
  listTasks(sessionId: string): Promise<TaskInfo[]>;

  sendMessage(sessionId: string, content: string): Promise<string>;
  cancelTask(taskId: string): Promise<void>;
  getTaskStatus(taskId: string): Promise<TaskInfo>;

  listProjects(): Promise<ProjectSummary[]>;
  createProject(name: string, rootPath: string): Promise<ProjectSummary>;
  openProject(projectId: string): Promise<void>;

  getWorkspace(): Promise<WorkspaceInfo | null>;
  /** Native folder picker; resolves to the chosen path or null if cancelled. */
  pickWorkspaceFolder(): Promise<string | null>;
  openWorkspace(path: string): Promise<WorkspaceInfo>;
  listFiles(path?: string): Promise<FileEntry[]>;
  readFile(path: string): Promise<string>;

  getAppInfo(): Promise<AppInfo>;
  onEvent(handler: (envelope: EventEnvelope) => void): () => void;

  /** Resolve a pending approval request (deny when in doubt). */
  respondApproval(approvalId: string, approved: boolean): Promise<void>;
  /** Subscribe to approval prompts; returns the unlisten callback. */
  onApproval(handler: (request: ApprovalRequest) => void): () => void;
}
