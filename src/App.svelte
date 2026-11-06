<script lang="ts">
  import { workspaceStore } from "./lib/stores/workspace";
  import { uiStore } from "./lib/stores/ui";
  import Sidebar from "./lib/components/layout/Sidebar.svelte";
  import EditorPanel from "./lib/components/layout/EditorPanel.svelte";
  import MetadataPanel from "./lib/components/layout/MetadataPanel.svelte";
  import Toast from "./lib/components/shared/Toast.svelte";
  import CommandPalette from "./lib/components/shared/CommandPalette.svelte";

  let { leftSidebarOpen, rightSidebarOpen, focusMode } = $derived($uiStore);
  let { root, selectedFile } = $derived($workspaceStore);

  /**
   * True when the event target is a text-entry surface: a form field
   * or an element inside a `contenteditable` region (the writing
   * surface is `contenteditable="plaintext-only"`).
   *
   * Shortcuts that toggle panels must not fire while the user is
   * typing — inside the editor, Ctrl+B / Ctrl+I mean bold / italic.
   */
  function isEditableTarget(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el || typeof el.tagName !== "string") return false;
    const tag = el.tagName.toLowerCase();
    if (tag === "input" || tag === "textarea") return true;
    if (el.isContentEditable) return true;
    // Fallback for engines that don't report `isContentEditable` for
    // every editable mode (e.g. `plaintext-only`).
    return typeof el.closest === "function"
      ? el.closest(
          '[contenteditable=""], [contenteditable="true"], [contenteditable="plaintext-only"]'
        ) !== null
      : false;
  }

  function handleKeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    // Ctrl+K opens the command palette, Ctrl+/ the shortcuts reference.
    // Both work while typing: neither combination means anything in the editor.
    if (mod && !e.shiftKey && !e.altKey && (e.key === "k" || e.key === "K")) {
      e.preventDefault();
      if ($uiStore.paletteOpen) uiStore.closePalette();
      else uiStore.openPalette();
      return;
    }
    if (mod && !e.shiftKey && !e.altKey && e.key === "/") {
      e.preventDefault();
      if ($uiStore.helpOpen) uiStore.closeHelp();
      else uiStore.openHelp();
      return;
    }
    // Panel toggles must not act behind an open dialog.
    if ($uiStore.paletteOpen || $uiStore.helpOpen) return;
    // Escape exits focus mode
    if (e.key === "Escape" && focusMode) {
      uiStore.toggleFocusMode();
    }
    // Ctrl+Shift+F toggles focus mode
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "F" || e.key === "f")) {
      e.preventDefault();
      uiStore.toggleFocusMode();
    }
    // Ctrl+Shift+P toggles preview
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "P" || e.key === "p")) {
      e.preventDefault();
      uiStore.togglePreview();
    }
    // Ctrl+B toggles file explorer
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === "B" || e.key === "b")) {
      // Don't hijack if the user is typing (form fields or the
      // contenteditable writing surface, where Ctrl+B means bold).
      if (isEditableTarget(e.target)) return;
      e.preventDefault();
      uiStore.toggleLeftSidebar();
    }
    // Ctrl+I toggles metadata panel
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === "I" || e.key === "i")) {
      if (isEditableTarget(e.target)) return;
      e.preventDefault();
      uiStore.toggleRightSidebar();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-shell" class:app-focus={focusMode && !!selectedFile}>
  {#if root && !focusMode}
    <Sidebar open={leftSidebarOpen} />
  {/if}

  <main class="app-main">
    <EditorPanel />
  </main>

  {#if root && !focusMode}
    <MetadataPanel open={rightSidebarOpen} />
  {/if}
</div>

<Toast />
<CommandPalette />

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--color-chrome);
    font-family: var(--font-ui);
    color: var(--color-ink);
    transition: background 0.2s ease;
  }

  .app-focus {
    background: var(--color-paper);
  }

  .app-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
</style>
