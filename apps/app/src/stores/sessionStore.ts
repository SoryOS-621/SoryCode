import { createStore, useStore } from "./createStore";

interface SessionState {
  selectedSessionId: string | null;
}

export const sessionStore = createStore<SessionState>({ selectedSessionId: null });

export function selectSession(sessionId: string | null): void {
  sessionStore.setState({ selectedSessionId: sessionId });
}

export function useSelectedSessionId(): string | null {
  return useStore(sessionStore).selectedSessionId;
}
