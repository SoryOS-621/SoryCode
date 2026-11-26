import { createStore, useStore } from "./createStore";

interface UiState {
  helpOpen: boolean;
  sidebarCollapsed: boolean;
}

export const uiStore = createStore<UiState>({
  helpOpen: false,
  sidebarCollapsed: false,
});

export function setHelpOpen(open: boolean): void {
  uiStore.setState((state) => ({ ...state, helpOpen: open }));
}

export function toggleSidebar(): void {
  uiStore.setState((state) => ({ ...state, sidebarCollapsed: !state.sidebarCollapsed }));
}

export function useUiState(): UiState {
  return useStore(uiStore);
}
