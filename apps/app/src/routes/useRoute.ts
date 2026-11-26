import { useCallback, useEffect, useState } from "react";
import { ROUTES, type RouteId } from "./index";

function parseHash(): RouteId {
  if (typeof window === "undefined") return "chat";
  const raw = window.location.hash.replace(/^#\/?/, "");
  return ROUTES.some((route) => route.id === raw) ? (raw as RouteId) : "chat";
}

export function useRoute(): [RouteId, (route: RouteId) => void] {
  const [route, setRoute] = useState<RouteId>(parseHash);

  useEffect(() => {
    if (!window.location.hash) window.history.replaceState(null, "", "#/chat");
    const onChange = () => setRoute(parseHash());
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  const navigate = useCallback((next: RouteId) => {
    window.location.hash = `/${next}`;
    setRoute(next);
  }, []);

  return [route, navigate];
}
