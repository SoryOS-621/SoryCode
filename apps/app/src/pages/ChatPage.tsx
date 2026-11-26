import { useEffect } from "react";
import { useEngine } from "../app/providers/EngineProvider";
import { ChatView } from "../features/chat/ChatView";
import { selectSession, useSelectedSessionId } from "../stores/sessionStore";
import { setHelpOpen } from "../stores/uiStore";
import type { RouteId } from "../routes";

export function ChatPage({ onNavigate }: { onNavigate: (route: RouteId) => void }) {
  const { createSession, sessions } = useEngine();
  const sessionId = useSelectedSessionId();

  useEffect(() => {
    if (!sessionId && sessions.length > 0) selectSession(sessions[0]!.id);
  }, [sessionId, sessions]);

  return (
    <ChatView
      sessionId={sessionId}
      onNewSession={() => {
        void createSession();
      }}
      onOpenSettings={() => onNavigate("settings")}
      onHelp={() => setHelpOpen(true)}
    />
  );
}
