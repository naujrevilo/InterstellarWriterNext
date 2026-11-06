import { writable } from "svelte/store";

interface UiState {
  leftSidebarOpen: boolean;
  rightSidebarOpen: boolean;
  focusMode: boolean;
  previewMode: boolean;
  paletteOpen: boolean;
  helpOpen: boolean;
}

function createUiStore() {
  const { subscribe, update } = writable<UiState>({
    leftSidebarOpen: true,
    rightSidebarOpen: true,
    focusMode: false,
    previewMode: false,
    paletteOpen: false,
    helpOpen: false,
  });

  return {
    subscribe,
    openPalette() {
      update((s) => ({ ...s, paletteOpen: true, helpOpen: false }));
    },
    closePalette() {
      update((s) => ({ ...s, paletteOpen: false }));
    },
    openHelp() {
      update((s) => ({ ...s, helpOpen: true, paletteOpen: false }));
    },
    closeHelp() {
      update((s) => ({ ...s, helpOpen: false }));
    },
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
