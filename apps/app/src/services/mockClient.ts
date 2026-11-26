import type {
  AppInfo,
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
import type { CoreClient } from "./coreClient";

const PROVIDERS: ProviderInfo[] = [
  {
    id: "opencode-zen",
    name: "opencode-zen",
    description: "OpenCode Zen (free models)",
    configured: true,
    freeOnly: true,
    defaultModel: "big-pickle",
    models: [
      "big-pickle",
      "mimo-v2.6-flash-free",
      "deepseek-v4.1-flash-free",
      "glm-5.3-flash-free",
      "nemotron-3.5-lightning-free",
    ],
  },
  {
    id: "openai",
    name: "openai",
    description: "OpenAI",
    configured: false,
    freeOnly: false,
    defaultModel: "gpt-4o-mini",
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4.1", "gpt-4.1-mini", "o3-mini"],
  },
  {
    id: "gemini",
    name: "gemini",
    description: "Google Gemini",
    configured: false,
    freeOnly: false,
    defaultModel: "gemini-2.0-flash",
    models: ["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
  },
  {
    id: "mistral",
    name: "mistral",
    description: "Mistral AI",
    configured: false,
    freeOnly: false,
    defaultModel: "mistral-small-latest",
    models: ["mistral-large-latest", "mistral-small-latest", "codestral-latest"],
  },
  {
    id: "groq",
    name: "groq",
    description: "Groq",
    configured: false,
    freeOnly: false,
    defaultModel: "llama-3.3-70b-versatile",
    models: ["llama-3.3-70b-versatile", "llama-3.1-8b-instant"],
  },
  {
    id: "nvidia",
    name: "nvidia",
    description: "NVIDIA NIM",
    configured: false,
    freeOnly: false,
    defaultModel: "nvidia/nemotron-3-super-120b-a12b",
    models: ["nvidia/nemotron-3-super-120b-a12b", "meta/llama-3.3-70b-instruct"],
  },
  {
    id: "openrouter",
    name: "openrouter",
    description: "OpenRouter (all models)",
    configured: false,
    freeOnly: false,
    defaultModel: "openai/gpt-4o-mini",
    models: ["openai/gpt-4o-mini", "anthropic/claude-3.5-sonnet"],
  },
  {
    id: "openrouter-free",
    name: "openrouter-free",
    description: "OpenRouter Free models",
    configured: true,
    freeOnly: true,
    defaultModel: "openrouter/free",
    models: ["openrouter/free", "meta-llama/llama-3.3-70b-instruct:free"],
  },
];

const now = () => new Date().toISOString();
const id = () => crypto.randomUUID();

function envelope(event: EventEnvelope["event"]): EventEnvelope {
  return { id: id(), timestamp: now(), event };
}

export function createMockClient(): CoreClient {
  let providers = PROVIDERS.map((provider) => ({ ...provider }));
  let selection: Selection = {
    provider: "opencode-zen",
    model: "big-pickle",
    models: providers[0]!.models,
  };
  let sessions: SessionSummary[] = [];
  let projects: ProjectSummary[] = [];
  let workspace: WorkspaceInfo = {
    path: "/home/sory/Desktop/Assistant-Sory",
    name: "Assistant-Sory",
    isGit: false,
    gitBranch: null,
  };
  const history = new Map<string, HistoryMessage[]>();
  const tasks = new Map<string, TaskInfo>();
  const listeners = new Set<(envelope: EventEnvelope) => void>();

  const emit = (event: EventEnvelope["event"]) => {
    const wrapped = envelope(event);
    for (const listener of listeners) listener(wrapped);
  };

  const ensureSession = (sessionId: string): SessionSummary => {
    let session = sessions.find((item) => item.id === sessionId);
    if (!session) {
      session = {
        id: sessionId,
        projectId: null,
        title: "New session",
        createdAt: now(),
        updatedAt: now(),
      };
      sessions = [session, ...sessions];
    }
    return session;
  };

  const client: CoreClient = {
    async bootstrap(): Promise<BootstrapPayload> {
      return { providers, selection, workspace, sessions };
    },
    async listProviders() {
      return providers;
    },
    async getSelection() {
      return selection;
    },
    async selectProvider(provider) {
      const preset = providers.find((item) => item.id === provider);
      if (!preset) throw new Error(`unknown provider: ${provider}`);
      selection = {
        provider: preset.id,
        model: preset.defaultModel,
        models: preset.models,
      };
      return selection;
    },
    async selectModel(model) {
      selection = { ...selection, model };
      return selection;
    },
    async connectProvider(provider, apiKey) {
      providers = providers.map((item) =>
        item.id === provider ? { ...item, configured: apiKey.trim().length > 0 || item.freeOnly } : item,
      );
      return selection;
    },

    async createSession(title) {
      const session: SessionSummary = {
        id: id(),
        projectId: null,
        title: title || "New session",
        createdAt: now(),
        updatedAt: now(),
      };
      sessions = [session, ...sessions];
      history.set(session.id, []);
      return session;
    },
    async listSessions() {
      return sessions;
    },
    async getHistory(sessionId) {
      return history.get(sessionId) ?? [];
    },
    async clearHistory(sessionId) {
      history.set(sessionId, []);
    },
    async listTasks(sessionId) {
      return [...tasks.values()].filter((task) => task.sessionId === sessionId);
    },

    async sendMessage(sessionId, content) {
      const session = ensureSession(sessionId);
      const messages = history.get(sessionId) ?? [];
      messages.push({ role: "user", content, toolCallId: null, toolCalls: [] });
      history.set(sessionId, messages);

      const taskId = id();
      const task: TaskInfo = {
        id: taskId,
        sessionId,
        description: content,
        status: "running",
        createdAt: now(),
        updatedAt: now(),
      };
      tasks.set(taskId, task);
      session.updatedAt = now();

      emit({
        type: "task_started",
        project_id: null,
        session_id: sessionId,
        task_id: taskId,
      });

      const reply =
        "This is the browser preview client. The real SoryCode engine runs inside the Tauri desktop shell.\n\n" +
        "Your request was:\n" +
        `> ${content}\n\n` +
        "Connect a provider in Settings, then run the desktop app to stream responses from the Rust core.";

      let index = 0;
      const chunkSize = 8;
      const timer = window.setInterval(() => {
        if (index >= reply.length) {
          window.clearInterval(timer);
          messages.push({ role: "assistant", content: reply, toolCallId: null, toolCalls: [] });
          history.set(sessionId, messages);
          task.status = "completed";
          task.updatedAt = now();
          tasks.set(taskId, task);
          emit({
            type: "task_completed",
            session_id: sessionId,
            task_id: taskId,
            result: reply,
          });
          return;
        }
        const delta = reply.slice(index, index + chunkSize);
        index += chunkSize;
        emit({ type: "message_delta", session_id: sessionId, task_id: taskId, delta });
      }, 18);

      return taskId;
    },
    async cancelTask(taskId) {
      const task = tasks.get(taskId);
      if (task) {
        task.status = "cancelled";
        task.updatedAt = now();
        tasks.set(taskId, task);
        emit({ type: "task_cancelled", session_id: task.sessionId, task_id: taskId });
      }
    },
    async getTaskStatus(taskId) {
      const task = tasks.get(taskId);
      if (!task) throw new Error("task not found");
      return task;
    },

    async listProjects() {
      return projects;
    },
    async createProject(name, rootPath) {
      const project: ProjectSummary = {
        id: id(),
        name,
        rootPath,
        createdAt: now(),
        updatedAt: now(),
      };
      projects = [project, ...projects];
      return project;
    },
    async openProject() {
      /* preview: nothing to open */
    },

    async getWorkspace() {
      return workspace;
    },
    async pickWorkspaceFolder() {
      return null;
    },
    async openWorkspace(path) {
      workspace = {
        path,
        name: path.split("/").filter(Boolean).pop() ?? path,
        isGit: false,
        gitBranch: null,
      };
      return workspace;
    },
    async listFiles(path = "") {
      const base: FileEntry[] = [
        { path: "AGENTS.md", name: "AGENTS.md", isDir: false },
        { path: "ARCHITECTURE.md", name: "ARCHITECTURE.md", isDir: false },
        { path: "Cargo.toml", name: "Cargo.toml", isDir: false },
        { path: "apps", name: "apps", isDir: true },
        { path: "apps/app", name: "app", isDir: true },
        { path: "apps/desktop", name: "desktop", isDir: true },
        { path: "apps/ui", name: "ui", isDir: true },
        { path: "crates", name: "crates", isDir: true },
        { path: "docs", name: "docs", isDir: true },
      ];
      if (!path) return base;
      return base.filter((entry) => entry.path.startsWith(path));
    },
    async readFile(path) {
      return `// ${path}\n// Preview content from the browser mock client.\n`;
    },

    async getAppInfo() {
      return {
        version: "0.1.0",
        dataPath: "~/.sorycode/desktop/state.json",
        credentialsPath: "~/.sorycode/credentials.json",
      } as AppInfo;
    },
    onEvent(handler) {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },

    async respondApproval() {
      /* preview: the mock engine never asks for permission */
    },
    onApproval() {
      return () => undefined;
    },
  };

  return client;
}
