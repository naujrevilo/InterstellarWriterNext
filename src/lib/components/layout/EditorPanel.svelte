<script lang="ts">
  import { workspaceStore } from "../../stores/workspace";
  import { documentStore, canUndo, canRedo } from "../../stores/document";
  import { uiStore } from "../../stores/ui";
  import { preferencesStore } from "../../stores/preferences";
  import { setSaveHandler } from "../../features/commands/registry";
  import { githubSyncStore, githubTokenStore } from "../../stores/github";
  import { notifications } from "../../stores/notifications";
  import { toRelativePath, basenameNoExt } from "../../utils/paths";
  import { ensureImport, componentNameFromPath } from "../../utils/mdxImport";
  import WritingSurface from "../editor/WritingSurface.svelte";
  import FontSelector from "../editor/FontSelector.svelte";
  import SyncButton from "../editor/SyncButton.svelte";
  import GitHubSettingsModal from "../editor/GitHubSettingsModal.svelte";
  import Preview from "../preview/Preview.svelte";

  let { root, selectedFile, loading, error: wsError } = $derived($workspaceStore);
  let doc = $derived($documentStore);
  let undoable = $derived($canUndo);
  let redoable = $derived($canRedo);
  let { previewMode, focusMode, leftSidebarOpen, rightSidebarOpen } = $derived($uiStore);
  let { editorFont } = $derived($preferencesStore);
  const AUTOSAVE_DELAY_MS = 2500;

  let settingsOpen = $state(false);
  let hasGithubToken = $derived($githubTokenStore.configured);
  let lastSavedAt = $state<number | null>(null);
  let saveFlash = $state(false);

  let manualPath = $state("");
  let writingSurfaceRef: { insertAtCaret: (text: string, options?: { caretOffset?: number }) => void } | null =
    $state(null);

  $effect(() => {
    if (selectedFile && root) {
      documentStore.load(selectedFile.relativePath, selectedFile.kind);
    }
  });

  $effect(() => {
    // Startup: move a legacy token out of localStorage and into the
    // operating-system credential store, then ask the backend whether a
    // token is configured.
    void (async () => {
      await preferencesStore.migrateLegacyToken();
      await githubTokenStore.refresh();
    })();
  });

  // Quiet autosave: waits for a pause in typing, never interrupts.
  $effect(() => {
    if (!$preferencesStore.autosave || !doc.dirty || !doc.relativePath) return;
    void doc.body;
    void doc.frontmatter;
    const timer = setTimeout(() => {
      void documentStore.save({ quiet: true }).then((written) => {
        if (written) lastSavedAt = Date.now();
      });
    }, AUTOSAVE_DELAY_MS);
    return () => clearTimeout(timer);
  });

  function handleInput(value: string) {
    documentStore.updateBody(value);
  }

  async function handleSave() {
    if (!root) return;
    if (!doc.dirty) return;
    await documentStore.save();
    saveFlash = true;
    setTimeout(() => (saveFlash = false), 1200);
    lastSavedAt = Date.now();
    // Re-scan so the sidebar reflects any derived metadata changes
    // (most importantly `isDraft`, which is peeked at scan time and
    // doesn't update when the user toggles the field in the panel).
    void workspaceStore.refresh();
    if ($preferencesStore.syncOnSave && $githubTokenStore.configured) {
      void githubSyncStore.sync();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      handleSave();
    }
  }

  $effect(() => {
    setSaveHandler(handleSave);
    return () => setSaveHandler(null);
  });

  /**
   * Open a native file picker, compute a workspace-relative path from
   * the current file's directory, and insert a markdown image snippet
   * at the caret. We use plain `![alt](path)` because Astro's `.mdx`
   * supports it natively, and the user can always promote it to the
   * `<Image />` component by hand if they want optimization.
   */
  async function handleImageClick() {
    if (!root || !selectedFile) {
      notifications.error("Open a workspace and a file first");
      return;
    }
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const picked = await openDialog({
        multiple: false,
        directory: false,
        title: "Insert image",
        filters: [
          {
            name: "Images",
            extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg", "avif"],
          },
        ],
      });
      if (!picked || typeof picked !== "string") return;

      const rel = toRelativePath(root, selectedFile.relativePath, picked);
      const alt = basenameNoExt(picked);
      const isMdx = selectedFile.kind === "mdx";
      const componentPath = $preferencesStore.figureComponentPath.trim();

      // .md → always plain markdown image.
      // .mdx without a configured component path → also plain markdown
      //   (works in MDX too, just not optimized).
      // .mdx with a configured component path → insert
      //   `<Component src="..." alt="..." />` at the caret, and make
      //   sure the matching `import` exists at the top of the file.
      if (isMdx && componentPath) {
        const componentName = componentNameFromPath(componentPath);
        const importStatement = `import ${componentName} from "${componentPath}";`;
        const componentTag =
          `<${componentName}\n  src="${rel}"\n  alt="${alt}"\n  caption="${alt}"\n  title="${alt}"\n/>`;

        // Ensure the import is present at the top of the body.
        const { body: withImport, inserted, alreadyPresent } = ensureImport(
          doc.body,
          importStatement
        );
        if (inserted || !alreadyPresent) {
          documentStore.updateBody(withImport);
        }
        // Place the component at the caret. The import was added
        // (or already existed) at the top; we insert the JSX where
        // the user was typing.
        writingSurfaceRef?.insertAtCaret(componentTag, {
          caretOffset: -2,
        });
        if (inserted) {
          notifications.success(
            `Added \`import ${componentName}\` and inserted <${componentName}/>`
          );
        }
      } else {
        // Plain markdown image — works for .md and as a fallback for
        // .mdx when the user hasn't configured a wrapper component.
        const snippet = `![${alt}](${rel})`;
        writingSurfaceRef?.insertAtCaret(snippet, { caretOffset: -1 });
      }
    } catch (e: any) {
      notifications.error(`Image picker failed: ${e?.message ?? e}`);
    }
  }

  async function handleManualOpen() {
    const path = manualPath.trim();
    if (path) {
      await workspaceStore.openWorkspaceAtPath(path);
    }
  }

  function handleManualKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      handleManualOpen();
    }
  }

  async function handleBrowse() {
    await workspaceStore.openWorkspace();
  }

  function handleCancel() {
    workspaceStore.cancel();
  }

  function selectedDisplay(file: typeof selectedFile): string {
    if (!file) return "";
    if (file.titleHint) return file.titleHint;
    if (typeof file.relativePath === "string" && file.relativePath.length > 0) {
      return file.relativePath.split('/').pop() ?? file.relativePath;
    }
    return "(unnamed)";
  }
</script>

<div class="editor-shell" class:editor-focus={focusMode}>
  <!-- Toolbar — collapses to nothing in focus mode -->
  {#if !focusMode}
    <header class="toolbar">
      <div class="toolbar-left">
        <button
          onclick={() => uiStore.toggleLeftSidebar()}
          class="icon-btn"
          class:icon-btn-panel-on={leftSidebarOpen}
          aria-label="Toggle file explorer"
          title="Toggle files (Ctrl+B)"
        >
          <svg width="17" height="17" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <rect x="2" y="3" width="12" height="10" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
            <line x1="5.5" y1="3" x2="5.5" y2="13" stroke="currentColor" stroke-width="1.2"/>
          </svg>
        </button>

        {#if selectedFile}
          <div class="toolbar-path" title={selectedFile.relativePath}>
            {#if selectedFile.collection}
              <span class="toolbar-path-collection">{selectedFile.collection}</span>
              <span class="toolbar-path-sep" aria-hidden="true">/</span>
            {/if}
            <span class="toolbar-path-name">{selectedDisplay(selectedFile)}</span>
          </div>
        {/if}
      </div>

      <div class="toolbar-center">
        <FontSelector />
      </div>

      <div class="toolbar-right">
        <button
          onclick={() => documentStore.undo()}
          class="icon-btn"
          disabled={!undoable}
          title="Undo (Ctrl+Z)"
          aria-label="Undo"
          style={undoable
            ? ""
            : "opacity:0.4;cursor:not-allowed;"}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M3.5 7L1 4.5L3.5 2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M1 4.5H10C12.5 4.5 14.5 6.5 14.5 9C14.5 11.5 12.5 13.5 10 13.5H5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <button
          onclick={() => documentStore.redo()}
          class="icon-btn"
          disabled={!redoable}
          title="Redo (Ctrl+Y / Ctrl+Shift+Z)"
          aria-label="Redo"
          style={redoable
            ? ""
            : "opacity:0.4;cursor:not-allowed;"}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M12.5 7L15 4.5L12.5 2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M15 4.5H6C3.5 4.5 1.5 6.5 1.5 9C1.5 11.5 3.5 13.5 6 13.5H11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>

        <button
          onclick={handleSave}
          class="primary-btn"
          class:primary-btn-flash={saveFlash}
          class:primary-btn-dirty={doc.dirty}
          disabled={!doc.dirty}
          title={doc.dirty ? "Save changes (Ctrl+S)" : "No changes to save"}
          style={doc.dirty
            ? "display:inline-flex;align-items:center;justify-content:center;height:32px;padding:0 18px;font-size:13px;font-weight:600;color:white;background:#6D28D9;border:1px solid #6D28D9;border-radius:6px;cursor:pointer;box-shadow:0 0 0 3px rgba(109,40,217,0.15);"
            : saveFlash
              ? "display:inline-flex;align-items:center;justify-content:center;height:32px;padding:0 18px;font-size:13px;font-weight:600;color:white;background:#166534;border:1px solid #166534;border-radius:6px;cursor:default;box-shadow:0 0 0 4px rgba(22,101,52,0.25);"
              : "display:inline-flex;align-items:center;justify-content:center;height:32px;padding:0 18px;font-size:13px;font-weight:600;color:#1F1B16;background:white;border:1px solid #D6CFC0;border-radius:6px;cursor:default;box-shadow:0 1px 2px rgba(0,0,0,0.04);"
          }
        >
          {doc.dirty ? "Save" : saveFlash ? "Saved ✓" : "Saved"}
        </button>

        <button
          onclick={() => preferencesStore.setAutosave(!$preferencesStore.autosave)}
          class="icon-btn icon-btn-text"
          aria-pressed={$preferencesStore.autosave}
          title={$preferencesStore.autosave
            ? "Autosave is on: saves 2.5 s after you stop typing. Click to turn off."
            : "Autosave is off. Click to save automatically after you stop typing."}
        >
          {$preferencesStore.autosave ? "Autosave on" : "Autosave off"}
        </button>

        <button
          onclick={() => uiStore.togglePreview()}
          class="icon-btn icon-btn-text"
          class:icon-btn-preview-on={previewMode}
          title={previewMode ? "Back to edit (Ctrl+Shift+P)" : "Preview (Ctrl+Shift+P)"}
        >
          {previewMode ? "Edit" : "Preview"}
        </button>

        <SyncButton onRequestToken={() => (settingsOpen = true)} />

        <button
          onclick={() => (settingsOpen = true)}
          class="icon-btn"
          aria-label="GitHub settings"
          title="GitHub settings"
        >
          <svg width="15" height="15" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.39v-1.36c-2.22.48-2.69-1.07-2.69-1.07-.36-.92-.89-1.16-.89-1.16-.73-.5.05-.49.05-.49.81.06 1.23.83 1.23.83.72 1.23 1.88.87 2.34.67.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.13 0 0 .67-.21 2.2.82a7.66 7.66 0 0 1 4 0c1.53-1.03 2.2-.82 2.2-.82.44 1.11.16 1.93.08 2.13.51.56.82 1.28.82 2.15 0 3.07-1.87 3.74-3.65 3.94.29.25.54.74.54 1.48v2.2c0 .22.15.47.55.39A8 8 0 0 0 8 0z" />
          </svg>
        </button>

        <button
          onclick={() => uiStore.toggleFocusMode()}
          class="icon-btn"
          class:icon-btn-focus-on={focusMode}
          aria-label={focusMode ? "Exit focus mode" : "Enter focus mode"}
          title="Focus mode (Ctrl+Shift+F)"
        >
          {#if focusMode}
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.1" opacity="0.4"/>
            </svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <circle cx="8" cy="8" r="3" stroke="currentColor" stroke-width="1.2"/>
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="0.9" opacity="0.4"/>
            </svg>
          {/if}
        </button>

        <button
          onclick={() => uiStore.toggleRightSidebar()}
          class="icon-btn"
          class:icon-btn-panel-on={rightSidebarOpen}
          aria-label="Toggle metadata panel"
          title="Toggle metadata (Ctrl+I)"
        >
          <svg width="17" height="17" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <rect x="2" y="3" width="12" height="10" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
            <line x1="10.5" y1="3" x2="10.5" y2="13" stroke="currentColor" stroke-width="1.2"/>
          </svg>
        </button>
      </div>
    </header>
  {/if}

  <!-- Editor area -->
  <div class="editor-body">
    {#if !selectedFile}
      <div class="empty-state">
        <div class="empty-state-inner">
          <div class="empty-state-mark" aria-hidden="true">
            <svg width="56" height="56" viewBox="0 0 48 48" fill="none">
              <defs>
                <linearGradient id="mark-gradient" x1="0" y1="0" x2="48" y2="48" gradientUnits="userSpaceOnUse">
                  <stop offset="0" stop-color="#6D28D9" stop-opacity="0.7"/>
                  <stop offset="1" stop-color="#A78BFA" stop-opacity="0.4"/>
                </linearGradient>
              </defs>
              <path d="M14 8h14l8 8v24a2 2 0 0 1-2 2H14a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2Z" stroke="url(#mark-gradient)" stroke-width="1.4"/>
              <path d="M28 8v8h8" stroke="url(#mark-gradient)" stroke-width="1.4"/>
              <line x1="17" y1="22" x2="31" y2="22" stroke="url(#mark-gradient)" stroke-width="1.4" opacity="0.7"/>
              <line x1="17" y1="28" x2="31" y2="28" stroke="url(#mark-gradient)" stroke-width="1.4" opacity="0.6"/>
              <line x1="17" y1="34" x2="26" y2="34" stroke="url(#mark-gradient)" stroke-width="1.4" opacity="0.5"/>
            </svg>
          </div>
          <h1 class="empty-state-title">Interstellar Writer</h1>
          <p class="empty-state-subtitle">A quiet place for your words.</p>

          {#if wsError}
            <div class="empty-state-error">{wsError}</div>
          {/if}

          <div class="empty-state-actions">
            {#if loading}
              <div class="empty-state-loading" role="status" aria-live="polite">
                <span class="empty-state-spinner" aria-hidden="true"></span>
                <span>Opening workspace…</span>
              </div>
              <button onclick={handleCancel} class="secondary-btn">Cancel</button>
            {:else}
              <button
                onclick={handleBrowse}
                class="primary-btn primary-btn-lg"
                style="display: flex; width: 100%; height: 44px; padding: 0 24px; font-size: 14px; font-weight: 600; color: white; background: #1F1B16; border: 1px solid #1F1B16; border-radius: 8px; align-items: center; justify-content: center; cursor: pointer; box-shadow: 0 2px 4px rgba(0,0,0,0.1); margin-bottom: 4px;"
              >
                Open workspace
              </button>

              <div class="empty-state-divider">
                <span>or paste a path</span>
              </div>

              <div class="empty-state-manual">
                <input
                  type="text"
                  bind:value={manualPath}
                  onkeydown={handleManualKeydown}
                  placeholder="/ruta/al/workspace"
                  class="empty-state-input"
                  spellcheck="false"
                />
                <button
                  onclick={handleManualOpen}
                  class="secondary-btn"
                  disabled={!manualPath.trim()}
                >
                  Open
                </button>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {:else if previewMode}
      <div class="preview-container">
        <Preview body={doc.body} kind={doc.kind} />
      </div>
    {:else}
      <WritingSurface
        bind:this={writingSurfaceRef}
        value={doc.body}
        oninput={handleInput}
        onkeydown={handleKeydown}
        onImageClick={handleImageClick}
      />
    {/if}
  </div>
</div>

<GitHubSettingsModal bind:open={settingsOpen} />

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: var(--color-paper);
    transition: background 0.2s ease;
  }

  .editor-focus {
    background: var(--color-paper);
  }

  /* ===== Toolbar ===== */
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--color-chrome);
    border-bottom: 1px solid var(--color-rule);
    min-height: 44px;
    gap: 8px;
    flex-shrink: 0;
  }

  /* LEFT side is allowed to shrink so the path truncates instead of
     pushing the right side off-screen. */
  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  /* CENTER (font selector) and RIGHT (action buttons) keep their natural
     width; they should never wrap. If the window is too narrow, the
     center can be hidden via the responsive rule below. */
  .toolbar-center {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    flex-shrink: 0;
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
    flex-shrink: 0;
  }

  .toolbar-path {
    display: flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-muted);
    padding: 0 4px;
    min-width: 0;
    overflow: hidden;
    flex: 0 1 auto;
  }

  .toolbar-path-collection {
    color: var(--color-accent);
    font-weight: 500;
    flex-shrink: 0;
  }

  .toolbar-path-sep {
    opacity: 0.5;
    flex-shrink: 0;
  }

  .toolbar-path-name {
    color: var(--color-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    max-width: 220px;
    flex: 0 1 auto;
  }

  /* When the window is narrow, hide the font selector so the path and
     the action buttons don't collide. The shortcut is documented in
     the empty state. */
  @media (max-width: 1100px) {
    .toolbar-center {
      display: none;
    }
  }

  @media (max-width: 820px) {
    .toolbar-path-name {
      max-width: 120px;
    }
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    min-width: 32px;
    padding: 0 9px;
    color: var(--color-ink-soft);
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 6px;
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }

  .icon-btn:hover {
    color: var(--color-ink);
    background: #fafaf9;
    border-color: var(--color-rule-strong);
  }

  /* Distinct, semantic colors per toolbar toggle so the user can tell
     at a glance which mode they're in. None of them competes with
     the Save button's violet-dirty state. */
  .icon-btn-preview-on {
    color: #166534;
    background: #DCFCE7;
    border-color: rgba(22, 101, 52, 0.35);
    box-shadow: 0 0 0 3px rgba(22, 101, 52, 0.12);
  }
  .icon-btn-preview-on:hover {
    color: #14532D;
    background: #DCFCE7;
    border-color: rgba(22, 101, 52, 0.45);
  }

  .icon-btn-focus-on {
    color: #5B21B6;
    background: #EDE4FB;
    border-color: rgba(109, 40, 217, 0.35);
    box-shadow: 0 0 0 3px rgba(109, 40, 217, 0.16);
  }
  .icon-btn-focus-on:hover {
    color: #4C1D95;
    background: #EDE4FB;
    border-color: rgba(109, 40, 217, 0.45);
  }

  .icon-btn-panel-on {
    color: var(--color-ink);
    background: rgba(0, 0, 0, 0.07);
    border-color: var(--color-rule-strong);
  }
  .icon-btn-panel-on:hover {
    color: var(--color-ink);
    background: rgba(0, 0, 0, 0.09);
    border-color: var(--color-ink-soft);
  }

  .icon-btn-text {
    font-family: var(--font-ui);
  }

  .primary-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    padding: 0 16px;
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    color: white;
    background: var(--color-ink);
    border: 1px solid var(--color-ink);
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s, transform 0.12s, box-shadow 0.15s;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
  }

  .primary-btn:hover:not(:disabled) {
    background: #292524;
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .primary-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .primary-btn:disabled {
    background: white;
    color: var(--color-ink-soft);
    border-color: var(--color-rule-strong);
    box-shadow: none;
    cursor: default;
  }

  .primary-btn-dirty {
    background: var(--color-accent);
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px rgba(109, 40, 217, 0.15);
  }

  .primary-btn-dirty:hover:not(:disabled) {
    background: #5b21b6;
    border-color: #5b21b6;
  }

  .primary-btn-flash {
    background: #166534 !important;
    border-color: #166534 !important;
    box-shadow: 0 0 0 4px rgba(22, 101, 52, 0.25) !important;
  }

  .primary-btn-lg {
    display: flex;
    width: 100%;
    height: 44px;
    padding: 0 24px;
    font-size: 14px;
    font-weight: 600;
  }

  .secondary-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 30px;
    padding: 0 14px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    color: var(--color-ink);
    background: white;
    border: 1px solid var(--color-rule-strong);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }

  .secondary-btn:hover:not(:disabled) {
    border-color: var(--color-ink-soft);
    background: #fafaf9;
  }

  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ===== Editor body ===== */
  .editor-body {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  /* ===== Empty state ===== */
  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: var(--color-chrome);
  }

  .empty-state-inner {
    text-align: center;
    max-width: 420px;
    width: 100%;
  }

  .empty-state-mark {
    color: var(--color-muted);
    margin-bottom: 24px;
    display: flex;
    justify-content: center;
    opacity: 0.7;
  }

  .empty-state-title {
    font-family: var(--font-editor);
    font-size: 32px;
    font-weight: 300;
    color: var(--color-ink);
    margin: 0 0 6px;
    letter-spacing: -0.02em;
    line-height: 1.2;
  }

  .empty-state-subtitle {
    font-family: var(--font-editor);
    font-size: 15px;
    font-style: italic;
    color: var(--color-muted);
    margin: 0 0 32px;
  }

  .empty-state-error {
    background: rgba(220, 38, 38, 0.06);
    color: #b91c1c;
    border: 1px solid rgba(220, 38, 38, 0.18);
    border-radius: 6px;
    padding: 10px 14px;
    font-size: 12px;
    margin-bottom: 16px;
    text-align: left;
    font-family: var(--font-ui);
  }

  .empty-state-actions {
    display: flex;
    flex-direction: column;
    gap: 18px;
    align-items: stretch;
  }

  .empty-state-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 10px 20px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--color-ink);
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 6px;
  }

  .empty-state-spinner {
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 2px solid var(--color-rule);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .empty-state-spinner { animation: none; }
  }

  .empty-state-divider {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--color-ink-soft);
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 0 4px;
  }

  .empty-state-divider::before,
  .empty-state-divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--color-rule-strong);
  }

  .empty-state-manual {
    display: flex;
    gap: 8px;
  }

  .empty-state-input {
    flex: 1;
    height: 38px;
    padding: 0 14px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-ink);
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 6px;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .empty-state-input:focus {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.08);
  }

  .empty-state-input::placeholder {
    color: var(--color-muted);
  }

  /* ===== Preview ===== */
  .preview-container {
    flex: 1;
    overflow: hidden;
    background: var(--color-paper);
  }
</style>
