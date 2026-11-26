import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import type {
  AppInfo,
  ApprovalRequest,
  EventEnvelope,
  ProviderInfo,
  Selection,
  SessionSummary,
  WorkspaceInfo,
} from "@sorycode/contracts";
import { coreClient, type CoreClient } from "../../services";
import { selectSession } from "../../stores/sessionStore";

interface EngineContextValue {
  client: CoreClient;
  providers: ProviderInfo[];
  selection: Selection | null;
  workspace: WorkspaceInfo | null;
  sessions: SessionSummary[];
  appInfo: AppInfo | null;
  /** Approval prompts waiting for a decision, oldest first. */
  approvals: ApprovalRequest[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  refreshSessions: () => Promise<void>;
  selectProvider: (provider: string) => Promise<void>;
  selectModel: (model: string) => Promise<void>;
  connectProvider: (provider: string, apiKey: string) => Promise<void>;
  createSession: (title?: string) => Promise<SessionSummary>;
  openWorkspace: (path: string) => Promise<void>;
  /** Answer the oldest pending approval (or a specific one). */
  respondApproval: (approvalId: string, approved: boolean) => Promise<void>;
}

const EngineContext = createContext<EngineContextValue | null>(null);

export function EngineProvider({ children }: { children: ReactNode }) {
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const bootstrap = await coreClient.bootstrap();
      setProviders(bootstrap.providers);
      setSelection(bootstrap.selection);
      setWorkspace(bootstrap.workspace);
      setSessions(bootstrap.sessions);
      setAppInfo(await coreClient.getAppInfo());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  const refreshSessions = useCallback(async () => {
    try {
      setSessions(await coreClient.listSessions());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    return coreClient.onEvent((envelope: EventEnvelope) => {
      const type = envelope.event.type;
      if (type === "task_completed" || type === "task_failed" || type === "task_cancelled") {
        void refreshSessions();
      }
    });
  }, [refreshSessions]);

  // Approval prompts travel on their own channel: they must never be dropped,
  // otherwise the engine waits for its five-minute timeout and denies the call.
  useEffect(() => {
    return coreClient.onApproval((request) => {
      setApprovals((current) =>
        current.some((item) => item.approvalId === request.approvalId)
          ? current
          : [...current, request],
      );
    });
  }, []);

  const respondApproval = useCallback(async (approvalId: string, approved: boolean) => {
    setApprovals((current) => current.filter((item) => item.approvalId !== approvalId));
    try {
      await coreClient.respondApproval(approvalId, approved);
    } catch (cause) {
      // The request most likely timed out on the Rust side; surface it instead
      // of silently pretending the decision was applied.
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  const selectProvider = useCallback(async (provider: string) => {
    try {
      const next = await coreClient.selectProvider(provider);
      setSelection(next);
      setProviders(await coreClient.listProviders());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  const selectModel = useCallback(async (model: string) => {
    try {
      setSelection(await coreClient.selectModel(model));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  const connectProvider = useCallback(async (provider: string, apiKey: string) => {
    try {
      const next = await coreClient.connectProvider(provider, apiKey);
      setSelection(next);
      setProviders(await coreClient.listProviders());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  const createSession = useCallback(
    async (title = "New session") => {
      const session = await coreClient.createSession(title);
      setSessions((current) => [session, ...current]);
      selectSession(session.id);
      return session;
    },
    [],
  );

  const openWorkspace = useCallback(async (path: string) => {
    const next = await coreClient.openWorkspace(path);
    setWorkspace(next);
    await refreshSessions();
  }, [refreshSessions]);

  const value = useMemo<EngineContextValue>(
    () => ({
      client: coreClient,
      providers,
      selection,
      workspace,
      sessions,
      appInfo,
      approvals,
      loading,
      error,
      refresh,
      refreshSessions,
      selectProvider,
      selectModel,
      connectProvider,
      createSession,
      openWorkspace,
      respondApproval,
    }),
    [
      providers,
      selection,
      workspace,
      sessions,
      appInfo,
      approvals,
      loading,
      error,
      refresh,
      refreshSessions,
      selectProvider,
      selectModel,
      connectProvider,
      createSession,
      openWorkspace,
      respondApproval,
    ],
  );

  return <EngineContext.Provider value={value}>{children}</EngineContext.Provider>;
}

export function useEngine(): EngineContextValue {
  const context = useContext(EngineContext);
  if (!context) throw new Error("useEngine must be used inside <EngineProvider>");
  return context;
}
