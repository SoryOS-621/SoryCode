import { useEngine } from "../app/providers/EngineProvider";
import { WorkView } from "../features/work/WorkView";
import { useSelectedSessionId } from "../stores/sessionStore";
import type { RouteId } from "../routes";

export function WorkPage({ onNavigate }: { onNavigate: (route: RouteId) => void }) {
  const { createSession } = useEngine();
  const sessionId = useSelectedSessionId();
  return (
    <WorkView
      sessionId={sessionId}
      onOpenChat={() => onNavigate("chat")}
      onNewSession={() => {
        void createSession().then(() => onNavigate("chat"));
      }}
    />
  );
}
