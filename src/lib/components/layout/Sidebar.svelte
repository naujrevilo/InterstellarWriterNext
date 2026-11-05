<script lang="ts">
  import { workspaceStore } from "../../stores/workspace";
  import { panelWidths, PANEL_LIMITS } from "../../stores/panelWidths";
  import type { DocumentRef, CollectionInfo } from "../../types/workspace";
  import ResizeHandle from "./ResizeHandle.svelte";

  let { open = true }: { open?: boolean } = $props();
  let { files, selectedFile, collections, kind, root } = $derived($workspaceStore);

  let searchQuery = $state("");
  let activeCollection = $state<string | null>(null);

  // Filter files by search query (title or relativePath).
  let visibleFiles = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return files;
    return files.filter(
      (f) =>
        (f.titleHint?.toLowerCase().includes(q) ?? false) ||
        f.relativePath.toLowerCase().includes(q)
    );
  });

  // Group files by collection. Standalone files without a collection
  // (e.g. .mmd/.mermaid diagrams) are gathered under a synthetic group.
  let groups = $derived.by(() => {
    const out: Array<{ collection: CollectionInfo; items: DocumentRef[] }> = [];
    for (const col of collections) {
      const items = visibleFiles.filter((f) => f.collection === col.name);
      if (items.length > 0) {
        out.push({ collection: col, items });
      }
    }
    const ungrouped = visibleFiles.filter((f) => !f.collection);
    if (ungrouped.length > 0) {
      out.push({
        collection: { name: "Other", relativePath: "", documentCount: ungrouped.length },
        items: ungrouped,
      });
    }
    return out;
  });

  async function selectFile(file: DocumentRef) {
    const selected = await workspaceStore.selectFile(file);
    if (selected) {
      activeCollection = file.collection ?? null;
    }
  }

  function kindIcon(kind: DocumentRef["kind"] | undefined): string {
    switch (kind) {
      case "markdown": return "MD";
      case "mdx": return "MX";
      case "mermaid-file": return "MM";
      default: return "··";
    }
  }

  function displayName(file: DocumentRef): string {
    if (file.titleHint) return file.titleHint;
    if (typeof file.relativePath === "string" && file.relativePath.length > 0) {
      return file.relativePath.split("/").pop() ?? file.relativePath;
    }
    return "(unnamed)";
  }

  // Friendly name from the root path (last segment).
  let workspaceName = $derived.by(() => {
    if (!root) return "";
    return root.split(/[\\/]/).filter(Boolean).pop() ?? root;
  });
</script>

<aside
  class="sidebar"
  class:sidebar-open={open}
  class:sidebar-closed={!open}
  style={open ? `width: ${$panelWidths.sidebar}px;` : ""}
>
  {#if open}
    <ResizeHandle
      side="left"
      width={$panelWidths.sidebar}
      onResize={panelWidths.setSidebar}
      min={PANEL_LIMITS.sidebar.min}
      max={PANEL_LIMITS.sidebar.max}
    />
  {/if}
  <header class="sidebar-header">
    <div class="sidebar-workspace">
      <span class="sidebar-workspace-dot" aria-hidden="true"></span>
      <span class="sidebar-workspace-name" title={root}>{workspaceName || "—"}</span>
    </div>
    <button
      onclick={() => workspaceStore.openWorkspace()}
      class="sidebar-action"
      aria-label="Open a different workspace"
      title="Open workspace"
    >
      <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h2.879a1.5 1.5 0 0 1 1.06.44l.622.62a.5.5 0 0 0 .354.147H12.5A1.5 1.5 0 0 1 14 5.707V12.5A1.5 1.5 0 0 1 12.5 14h-9A1.5 1.5 0 0 1 2 12.5v-8Z" stroke="currentColor" stroke-width="1.1"/>
      </svg>
    </button>
  </header>

  <div class="sidebar-search">
    <svg class="sidebar-search-icon" width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <circle cx="7" cy="7" r="4.5" stroke="currentColor" stroke-width="1.1"/>
      <path d="M10.5 10.5 14 14" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
    </svg>
    <input
      type="text"
      bind:value={searchQuery}
      placeholder="Search files…"
      class="sidebar-search-input"
      aria-label="Search files"
    />
    {#if searchQuery}
      <button
        onclick={() => (searchQuery = "")}
        class="sidebar-search-clear"
        aria-label="Clear search"
      >
        ×
      </button>
    {/if}
  </div>

  <nav class="sidebar-nav" aria-label="File explorer">
    {#if kind === "astro"}
      {#if groups.length === 0}
        <div class="sidebar-empty">
          <p>No content collections found.</p>
          <p class="sidebar-empty-hint">Add markdown under <code>src/content/&lt;name&gt;/</code>.</p>
        </div>
      {/if}
      {#each groups as group (group.collection.name)}
        <section class="sidebar-group" class:sidebar-group-open={activeCollection === null || activeCollection === group.collection.name}>
          <header class="sidebar-group-header">
            <span class="sidebar-group-name">{group.collection.name}</span>
            <span class="sidebar-group-count">{group.items.length}</span>
          </header>
          <ul class="sidebar-group-list">
            {#each group.items as file (file.relativePath)}
              <li>
                <button
                  onclick={() => selectFile(file)}
                  class="sidebar-file"
                  class:sidebar-file-active={selectedFile?.relativePath === file.relativePath}
                  class:sidebar-file-draft={file.isDraft}
                  title={file.relativePath}
                >
                  <span class="sidebar-file-kind">{kindIcon(file.kind)}</span>
                  <span class="sidebar-file-name">{displayName(file)}</span>
                  {#if file.isDraft}
                    <span class="sidebar-file-draft-badge" aria-label="Draft" title="Draft">DRAFT</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {:else}
      <ul class="sidebar-group-list sidebar-group-list-flat">
        {#each visibleFiles as file (file.relativePath)}
          <li>
            <button
              onclick={() => selectFile(file)}
              class="sidebar-file"
              class:sidebar-file-active={selectedFile?.relativePath === file.relativePath}
              class:sidebar-file-draft={file.isDraft}
              title={file.relativePath}
            >
              <span class="sidebar-file-kind">{kindIcon(file.kind)}</span>
              <span class="sidebar-file-name">{displayName(file)}</span>
              {#if file.isDraft}
                <span class="sidebar-file-draft-badge" aria-label="Draft" title="Draft">DRAFT</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </nav>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    background: var(--color-chrome);
    transition: width 0.18s ease, opacity 0.18s ease, transform 0.18s ease;
    overflow: hidden;
    min-width: 0;
    position: relative;
    flex-shrink: 0;
  }

  .sidebar-open {
    /* width is set inline by the panel-widths store */
    opacity: 1;
    transform: translateX(0);
  }

  .sidebar-closed {
    width: 0;
    opacity: 0;
    transform: translateX(-8px);
    pointer-events: none;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px 10px;
    gap: 8px;
  }

  .sidebar-workspace {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }

  .sidebar-workspace-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-accent);
    flex-shrink: 0;
    opacity: 0.7;
  }

  .sidebar-workspace-name {
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    color: var(--color-ink);
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sidebar-action {
    color: var(--color-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background 0.15s;
    flex-shrink: 0;
  }

  .sidebar-action:hover {
    color: var(--color-ink);
    background: rgba(0, 0, 0, 0.04);
  }

  .sidebar-search {
    position: relative;
    margin: 0 14px 8px;
    display: flex;
    align-items: center;
  }

  .sidebar-search-icon {
    position: absolute;
    left: 11px;
    color: var(--color-muted);
    pointer-events: none;
  }

  .sidebar-search-input {
    width: 100%;
    padding: 7px 26px 7px 32px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--color-ink);
    background: rgba(255, 255, 255, 0.6);
    border: 1px solid transparent;
    border-radius: 6px;
    outline: none;
    transition: background 0.15s, border-color 0.15s;
  }

  .sidebar-search-input::placeholder {
    color: var(--color-muted);
  }

  .sidebar-search-input:focus {
    background: white;
    border-color: var(--color-rule);
  }

  .sidebar-search-clear {
    position: absolute;
    right: 6px;
    background: transparent;
    border: none;
    color: var(--color-muted);
    cursor: pointer;
    font-size: 14px;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
  }

  .sidebar-search-clear:hover {
    color: var(--color-ink);
    background: rgba(0, 0, 0, 0.05);
  }

  .sidebar-nav {
    flex: 1;
    overflow-y: auto;
    padding: 4px 10px 16px;
  }

  .sidebar-group {
    margin-top: 12px;
  }

  .sidebar-group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 10px 6px;
  }

  .sidebar-group-name {
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-muted);
  }

  .sidebar-group-count {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-muted);
    background: rgba(0, 0, 0, 0.04);
    padding: 1px 6px;
    border-radius: 8px;
    min-width: 18px;
    text-align: center;
  }

  .sidebar-group-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .sidebar-group-list-flat {
    margin-top: 4px;
  }

  .sidebar-file {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 400;
    color: var(--color-ink);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
    overflow: hidden;
  }

  .sidebar-file:hover {
    background: rgba(0, 0, 0, 0.04);
    border-color: var(--color-rule);
  }

  .sidebar-file-active {
    background: var(--color-accent-soft);
    color: var(--color-ink);
    border-color: rgba(109, 40, 217, 0.3);
  }

  .sidebar-file-active:hover {
    background: var(--color-accent-soft);
  }

  .sidebar-file-kind {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 500;
    color: var(--color-muted);
    flex-shrink: 0;
    width: 22px;
    text-align: center;
    letter-spacing: 0.02em;
    background: rgba(0, 0, 0, 0.04);
    border-radius: 3px;
    padding: 1px 0;
  }

  .sidebar-file-active .sidebar-file-kind {
    color: var(--color-accent);
    background: rgba(124, 58, 237, 0.08);
  }

  .sidebar-file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .sidebar-file-draft .sidebar-file-name {
    color: var(--color-warning);
    font-style: italic;
  }

  .sidebar-file-draft-badge {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 600;
    color: #92400e;
    background: #fef3c7;
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
    letter-spacing: 0.04em;
  }

  .sidebar-file-draft.sidebar-file-active .sidebar-file-draft-badge {
    background: rgba(146, 64, 14, 0.18);
  }

  .sidebar-empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--color-muted);
    font-family: var(--font-ui);
    font-size: 13px;
  }

  .sidebar-empty p {
    margin: 0 0 4px;
  }

  .sidebar-empty-hint {
    font-size: 12px;
    opacity: 0.8;
  }

  .sidebar-empty code {
    font-family: var(--font-mono);
    font-size: 11px;
    background: rgba(0, 0, 0, 0.05);
    padding: 1px 4px;
    border-radius: 3px;
  }
</style>
