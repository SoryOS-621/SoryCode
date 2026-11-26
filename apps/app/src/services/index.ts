import type { CoreClient } from "./coreClient";
import { createMockClient } from "./mockClient";
import { createTauriClient } from "./tauriClient";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function createCoreClient(): CoreClient {
  return isTauri() ? createTauriClient() : createMockClient();
}

export const coreClient: CoreClient = createCoreClient();
export type { CoreClient } from "./coreClient";
