import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  BootstrapPayload,
  EventEnvelope,
  FileEntry,
  HistoryMessage,
  MessageRole,
  ProjectSummary,
  ProviderInfo,
  Selection,
  SessionSummary,
  TaskInfo,
  ToolCall,
  WorkspaceInfo,
} from "@sorycode/contracts";
import type { CoreClient } from "./coreClient";

const EVENT_CHANNEL = "sorycode://event";
const APPROVAL_CHANNEL = "sorycode://approval";

interface WireApproval {
  approvalId: string;
  action: string;
  reason: string;
}

interface WireMessage {
  role: MessageRole;
  content: string;
  tool_call_id: string | null;
  tool_calls: ToolCall[];
}

function fromWire(message: WireMessage): HistoryMessage {
  return {
    role: message.role,
    content: message.content,
    toolCallId: message.tool_call_id,
    toolCalls: message.tool_calls ?? [],
  };
}

export function createTauriClient(): CoreClient {
  return {
    bootstrap: () => invoke<BootstrapPayload>("bootstrap"),
    listProviders: () => invoke<ProviderInfo[]>("list_providers"),
    getSelection: () => invoke<Selection>("get_selection"),
    selectProvider: (provider) => invoke<Selection>("select_provider", { provider }),
    selectModel: (model) => invoke<Selection>("select_model", { model }),
    connectProvider: (provider, apiKey) =>
      invoke<Selection>("connect_provider", { provider, apiKey }),

    createSession: (title, projectId = null) =>
      invoke<SessionSummary>("create_session", { title, projectId }),
    listSessions: (projectId = null) =>
      invoke<SessionSummary[]>("list_sessions", { projectId }),
    getHistory: async (sessionId) => {
      const messages = await invoke<WireMessage[]>("get_history", { sessionId });
      return messages.map(fromWire);
    },
    clearHistory: (sessionId) => invoke<void>("clear_history", { sessionId }),
    listTasks: (sessionId) => invoke<TaskInfo[]>("list_tasks", { sessionId }),

    sendMessage: (sessionId, content) =>
      invoke<string>("send_message", { sessionId, content }),
    cancelTask: (taskId) => invoke<void>("cancel_task", { taskId }),
    getTaskStatus: (taskId) => invoke<TaskInfo>("get_task_status", { taskId }),

    listProjects: () => invoke<ProjectSummary[]>("list_projects"),
    createProject: (name, rootPath) =>
      invoke<ProjectSummary>("create_project", { name, rootPath }),
    openProject: (projectId) => invoke<void>("open_project", { projectId }),

    getWorkspace: () => invoke<WorkspaceInfo | null>("get_workspace"),
    pickWorkspaceFolder: () => invoke<string | null>("pick_workspace_folder"),
    openWorkspace: (path) => invoke<WorkspaceInfo>("open_workspace", { path }),
    listFiles: (path) =>
      invoke<FileEntry[]>("list_workspace_files", { path: path ?? null }),
    readFile: (path) => invoke<string>("read_workspace_file", { path }),

    getAppInfo: () => invoke<AppInfo>("get_app_info"),
    onEvent: (handler) => {
      let unlisten: UnlistenFn | undefined;
      let disposed = false;
      void listen<EventEnvelope>(EVENT_CHANNEL, (event) => handler(event.payload)).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
      return () => {
        disposed = true;
        unlisten?.();
      };
    },

    respondApproval: (approvalId, approved) =>
      invoke<void>("respond_approval", { approvalId, approved }),

    onApproval: (handler) => {
      let unlisten: UnlistenFn | undefined;
      let disposed = false;
      void listen<WireApproval>(APPROVAL_CHANNEL, (event) => handler(event.payload)).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
      return () => {
        disposed = true;
        unlisten?.();
      };
    },
  };
}
