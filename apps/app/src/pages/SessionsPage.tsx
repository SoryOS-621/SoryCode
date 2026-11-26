import { SessionsView } from "../features/sessions/SessionsView";
import type { RouteId } from "../routes";

export function SessionsPage({ onNavigate }: { onNavigate: (route: RouteId) => void }) {
  return <SessionsView onOpenChat={() => onNavigate("chat")} />;
}
