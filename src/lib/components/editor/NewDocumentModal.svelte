<script lang="ts">
  import type { CollectionInfo, DocumentRef, WorkspaceKind } from "../../types/workspace";
  import { workspaceStore } from "../../stores/workspace";

  let {
    open,
    kind,
    collections,
    files,
    initialCollection,
    onClose,
  }: {
    open: boolean;
    kind: WorkspaceKind;
    collections: CollectionInfo[];
    files: DocumentRef[];
    initialCollection: string | null;
    onClose: () => void;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let title = $state("");
  let filename = $state("");
  let filenameEdited = $state(false);
  let extension = $state<"md" | "mdx">("md");
  let collection = $state("");
  let directory = $state("");
  let submitting = $state(false);
  let error = $state("");

  function preferredExtension(selected: string): "md" | "mdx" {
    const items = files.filter((file) => file.collection === selected);
    const markdown = items.filter((file) => file.kind === "markdown").length;
    const mdx = items.filter((file) => file.kind === "mdx").length;
    return mdx > markdown ? "mdx" : "md";
  }

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      title = "";
      filename = "";
      filenameEdited = false;
      collection = initialCollection ?? collections[0]?.name ?? "";
      extension = kind === "astro" ? preferredExtension(collection) : "md";
      directory = "";
      error = "";
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function suggestFilename(value: string): string {
    return value.trim().toLowerCase().normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^\p{L}\p{N}]+/gu, "-")
      .replace(/^-+|-+$/g, "");
  }

  function handleTitleInput(event: Event) {
    title = (event.currentTarget as HTMLInputElement).value;
    if (!filenameEdited) filename = suggestFilename(title);
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (submitting) return;
    if (!title.trim() || !filename.trim()) {
      error = "Enter a title and a filename.";
      return;
    }
    if (kind === "astro" && !collection) {
      error = "Select a content collection.";
      return;
    }
    submitting = true;
    error = "";
    try {
      const created = await workspaceStore.createDocument({
        title: title.trim(),
        filename: `${filename.trim()}.${extension}`,
        collection: kind === "astro" ? collection : null,
        directory: directory.trim(),
      });
      if (created) onClose();
    } catch (e: unknown) {
      error = `Could not create document: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      submitting = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="new-document"
  aria-labelledby="new-document-title"
  oncancel={(event) => {
    if (submitting) event.preventDefault();
  }}
  onclose={() => {
    if (open) onClose();
  }}
>
  <form onsubmit={submit}>
    <header>
      <div>
        <h2 id="new-document-title">New document</h2>
        <p>Create a Markdown file and open it in the editor.</p>
      </div>
      <button type="button" class="close" aria-label="Close" disabled={submitting} onclick={onClose}>×</button>
    </header>

    {#if kind === "astro"}
      <label for="new-collection">Collection</label>
      <select
        id="new-collection"
        bind:value={collection}
        onchange={() => (extension = preferredExtension(collection))}
        required
        disabled={submitting}
      >
        {#if collections.length === 0}
          <option value="">No collections available</option>
        {/if}
        {#each collections as item (item.name)}
          <option value={item.name}>{item.name}</option>
        {/each}
      </select>
      <p class="hint">The file will be created inside the selected content collection.</p>
    {:else}
      <label for="new-directory">Folder inside workspace</label>
      <input id="new-directory" bind:value={directory} placeholder="Workspace root" disabled={submitting} />
      <p class="hint">Leave empty for the workspace root, or enter an existing relative folder.</p>
    {/if}

    <label for="new-title">Title</label>
    <input id="new-title" value={title} oninput={handleTitleInput} placeholder="A title for your document" required disabled={submitting} />

    <label for="new-filename">Filename</label>
    <div class="filename-row">
      <input id="new-filename" bind:value={filename} oninput={() => (filenameEdited = true)} placeholder="document-name" required disabled={submitting} />
      <select aria-label="File format" bind:value={extension} disabled={submitting}>
        <option value="md">.md</option>
        <option value="mdx">.mdx</option>
      </select>
    </div>
    {#if kind === "astro"}
      <p class="hint">The file is created immediately with only a title; it is not automatically marked as a draft. Complete the metadata required by your collection before publishing.</p>
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <footer>
      <button type="button" class="cancel" onclick={onClose} disabled={submitting}>Cancel</button>
      <button type="submit" class="create" disabled={submitting || (kind === "astro" && collections.length === 0)}>
        {submitting ? "Creating…" : "Create and edit"}
      </button>
    </footer>
  </form>
</dialog>

<style>
  .new-document {
    width: min(460px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    padding: 24px;
    border: 1px solid var(--color-rule);
    border-radius: 12px;
    background: var(--color-paper);
    box-shadow: 0 18px 60px rgba(31, 27, 22, 0.2);
    color: var(--color-ink);
    font-family: var(--font-ui);
  }
  .new-document::backdrop {
    background: rgba(31, 27, 22, 0.28);
    backdrop-filter: blur(2px);
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }
  h2 {
    margin: 0 0 5px;
    font-family: var(--font-editor);
    font-size: 23px;
    font-weight: 500;
  }
  header p, .hint {
    margin: 0;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .close {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 6px;
    color: var(--color-muted);
    background: transparent;
    cursor: pointer;
    font-size: 20px;
  }
  label {
    display: block;
    margin: 14px 0 6px;
    font-size: 12px;
    font-weight: 600;
  }
  input, select {
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    height: 38px;
    padding: 0 10px;
    border: 1px solid var(--color-rule-strong);
    border-radius: 6px;
    background: white;
    color: var(--color-ink);
    font: inherit;
    font-size: 13px;
  }
  input:focus, select:focus {
    outline: 2px solid var(--color-accent);
    outline-offset: 1px;
  }
  .filename-row {
    display: flex;
    gap: 8px;
  }
  .filename-row select {
    width: 82px;
    flex-shrink: 0;
  }
  .hint { margin-top: 6px; }
  .error {
    margin: 12px 0 0;
    color: #b91c1c;
    font-size: 13px;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 22px;
  }
  footer button {
    padding: 8px 14px;
    border: 1px solid var(--color-rule-strong);
    border-radius: 6px;
    background: white;
    color: var(--color-ink);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  footer .create {
    border-color: var(--color-ink);
    background: var(--color-ink);
    color: white;
  }
  button:disabled { opacity: 0.5; cursor: default; }
</style>
