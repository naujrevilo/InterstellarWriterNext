import { writable } from "svelte/store";

interface UiState {
  leftSidebarOpen: boolean;
  rightSidebarOpen: boolean;
  focusMode: boolean;
  previewMode: boolean;
}

function createUiStore() {
  const { subscribe, update } = writable<UiState>({
    leftSidebarOpen: true,
    rightSidebarOpen: true,
    focusMode: false,
    previewMode: false,
  });

  return {
    subscribe,
    toggleLeftSidebar() {
      update((s) => ({ ...s, leftSidebarOpen: !s.leftSidebarOpen }));
    },
    toggleRightSidebar() {
      update((s) => ({ ...s, rightSidebarOpen: !s.rightSidebarOpen }));
    },
    toggleFocusMode() {
      update((s) => ({ ...s, focusMode: !s.focusMode }));
    },
    togglePreview() {
      update((s) => ({ ...s, previewMode: !s.previewMode }));
    },
  };
}

export const uiStore = createUiStore();
