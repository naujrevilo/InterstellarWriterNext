<script lang="ts">
  import { workspaceStore } from "../../stores/workspace";
  import { documentStore } from "../../stores/document";
  import { panelWidths, PANEL_LIMITS } from "../../stores/panelWidths";
  import { resolveImage } from "../../api/tauri";
  import { getFieldsForCollection, type MetadataFieldDef } from "../../types/metadata";
  import ResizeHandle from "./ResizeHandle.svelte";

  let { open = true }: { open?: boolean } = $props();
  let { root } = $derived($workspaceStore);
  let { selectedFile } = $derived($workspaceStore);
  let doc = $derived($documentStore);

  let fields: MetadataFieldDef[] = $derived(
    selectedFile ? getFieldsForCollection(selectedFile.collection ?? null) : []
  );

  /** Map widget type → short label rendered as a chip. */
  function fieldTypeBadge(widget: MetadataFieldDef["widget"]): string {
    switch (widget) {
      case "text": return "TXT";
      case "textarea": return "¶";
      case "datetime": return "DATE";
      case "path": return "PATH";
      case "checkbox": return "BOOL";
      case "string-array": return "LIST";
      default: return "?";
    }
  }

  /** Map widget type → color for the type chip. */
  function fieldTypeColor(widget: MetadataFieldDef["widget"]): string {
    switch (widget) {
      case "text": return "#6366f1";      // indigo
      case "textarea": return "#0ea5e9";  // sky
      case "datetime": return "#f59e0b";   // amber
      case "path": return "#14b8a6";      // teal
      case "checkbox": return "#ec4899";   // pink
      case "string-array": return "#8b5cf6"; // violet
      default: return "var(--color-muted)";
    }
  }

  function updateField(key: string, value: unknown) {
    documentStore.updateFrontmatter({
      ...doc.frontmatter,
      [key]: value,
    });
  }

  function asText(value: unknown): string {
    if (value == null) return "";
    if (typeof value === "string") return value;
    if (typeof value === "number" || typeof value === "boolean") return String(value);
    if (Array.isArray(value)) return value.join(", ");
    return "";
  }

  function asDateInputValue(value: unknown): string {
    if (value == null) return "";
    if (value instanceof Date) return value.toISOString().slice(0, 10);
    if (typeof value === "string") {
      const iso = /^(\d{4}-\d{2}-\d{2})/.exec(value);
      if (iso) return iso[1];
      if (/^\d{4}-\d{2}-\d{2}$/.test(value)) return value;
      return "";
    }
    return "";
  }

  function asBool(value: unknown): boolean {
    return value === true;
  }

  function asStringArray(value: unknown): string[] {
    if (Array.isArray(value)) return value.filter((v): v is string => typeof v === "string");
    if (value == null) return [];
    if (typeof value === "string") return value ? [value] : [];
    return [];
  }

  function addArrayItem(key: string) {
    const current = asStringArray(doc.frontmatter[key]);
    updateField(key, [...current, ""]);
  }

  function removeArrayItem(key: string, index: number) {
    const current = asStringArray(doc.frontmatter[key]);
    updateField(key, current.filter((_, i) => i !== index));
  }

  function updateArrayItem(key: string, index: number, value: string) {
    const current = [...asStringArray(doc.frontmatter[key])];
    current[index] = value;
    updateField(key, current);
  }

  function handleArrayKey(e: KeyboardEvent, key: string, index: number) {
    if (e.key === "Enter") {
      e.preventDefault();
      addArrayItem(key);
      // Focus the new input on next tick
      requestAnimationFrame(() => {
        const inputs = document.querySelectorAll<HTMLInputElement>(
          `[data-array-key="${key}"]`
        );
        inputs[inputs.length - 1]?.focus();
      });
    } else if (e.key === "Backspace" && (e.target as HTMLInputElement).value === "") {
      e.preventDefault();
      removeArrayItem(key, index);
    }
  }

  function slugify(s: string): string {
    return s
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function fieldId(key: string): string {
    return `meta-${slugify(key)}`;
  }

  let fileName = $derived(
    selectedFile?.relativePath?.split(/[\\/]/).pop() ?? ""
  );

  let fileDir = $derived(
    selectedFile?.relativePath?.split(/[\\/]/).slice(0, -1).join("/") ?? ""
  );

  /**
   * Frontmatter keys we treat as "this is a path to an image" — they
   * get a thumbnail under the input. Mirrors the keys we look for in
   * the preview hero.
   */
  const IMAGE_FIELD_PATTERN = /^(image|cover|thumbnail|hero|banner|og[_-]?image|social[_-]?image)$/i;

  function isImageField(field: MetadataFieldDef): boolean {
    return field.widget === "path" && IMAGE_FIELD_PATTERN.test(field.key);
  }

  /**
   * Map of frontmatter key → resolved data: URL (or `null` if the path
   * is empty, invalid, or outside the workspace). We only fetch each
   * unique value once, cached in this map, and re-resolve when the
   * user types a new path.
   */
  let thumbnails: Record<string, string | null> = $state({});
  const thumbnailInflight = new Set<string>();

  $effect(() => {
    if (!root || !selectedFile) return;

    for (const field of fields) {
      if (!isImageField(field)) continue;

      const value = asText(doc.frontmatter[field.key]).trim();

      if (!value) {
        if (thumbnails[field.key] !== null) thumbnails[field.key] = null;
        continue;
      }

      // Skip data:/http(s) URLs — they don't need a backend roundtrip.
      if (
        value.startsWith("data:") ||
        value.startsWith("http://") ||
        value.startsWith("https://") ||
        value.startsWith("//")
      ) {
        if (thumbnails[field.key] !== value) thumbnails[field.key] = value;
        continue;
      }

      // Dedup: don't fire the same resolution twice while it's in flight.
      if (thumbnailInflight.has(value)) continue;
      thumbnailInflight.add(value);

      resolveImage(selectedFile.relativePath, value)
        .then(({ dataUrl }) => {
          thumbnailInflight.delete(value);
          thumbnails[field.key] = dataUrl;
        })
        .catch(() => {
          thumbnailInflight.delete(value);
          if (thumbnails[field.key] !== null) thumbnails[field.key] = null;
        });
    }
  });
</script>

<aside
  class="metadata"
  class:metadata-open={open}
  class:metadata-closed={!open}
  style={open ? `width: ${$panelWidths.metadata}px;` : ""}
>
  {#if open}
    <ResizeHandle
      side="right"
      width={$panelWidths.metadata}
      onResize={panelWidths.setMetadata}
      min={PANEL_LIMITS.metadata.min}
      max={PANEL_LIMITS.metadata.max}
    />
  {/if}
  <header class="metadata-header">
    <div class="metadata-header-row">
      <h2 class="metadata-title">Metadata</h2>
      {#if doc.dirty}
        <span class="metadata-dirty-dot" title="Unsaved changes"></span>
      {/if}
    </div>
    {#if selectedFile}
      <div class="metadata-context">
        <div class="metadata-context-collection">
          <span class="metadata-context-collection-dot" aria-hidden="true"></span>
          <span class="metadata-context-collection-name">{selectedFile.collection || "no collection"}</span>
        </div>
        <div class="metadata-context-file" title={selectedFile.relativePath}>
          <span class="metadata-context-file-name">{fileName}</span>
          {#if fileDir}
            <span class="metadata-context-file-dir">in {fileDir}</span>
          {/if}
        </div>
      </div>
    {/if}
  </header>

  <div class="metadata-scroll">
    {#if !selectedFile}
      <div class="metadata-empty">
        <div class="metadata-empty-illust" aria-hidden="true">
          <svg width="56" height="56" viewBox="0 0 56 56" fill="none">
            <rect x="10" y="6" width="32" height="44" rx="2" stroke="currentColor" stroke-width="1.2" opacity="0.4"/>
            <line x1="16" y1="18" x2="36" y2="18" stroke="currentColor" stroke-width="1.2" opacity="0.5"/>
            <line x1="16" y1="24" x2="36" y2="24" stroke="currentColor" stroke-width="1.2" opacity="0.4"/>
            <line x1="16" y1="30" x2="30" y2="30" stroke="currentColor" stroke-width="1.2" opacity="0.3"/>
            <circle cx="42" cy="44" r="11" fill="var(--color-paper)" stroke="currentColor" stroke-width="1.3" opacity="0.6"/>
            <path d="M42 38v6m0 3v.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" opacity="0.7"/>
          </svg>
        </div>
        <p class="metadata-empty-title">No file selected</p>
        <p class="metadata-empty-hint">Open a file from the sidebar to see and edit its metadata.</p>
      </div>
    {:else}
      <div class="metadata-body">
        {#each fields as field (field.key)}
          <div class="metadata-field" class:metadata-field-required={field.required}>
            <div class="metadata-field-head">
              <span
                class="metadata-field-typechip"
                style="color: {fieldTypeColor(field.widget)}; border-color: {fieldTypeColor(field.widget)}33; background: {fieldTypeColor(field.widget)}11;"
                title={`Field type: ${field.widget}`}
              >
                {fieldTypeBadge(field.widget)}
              </span>
              <label for={fieldId(field.key)} class="metadata-label">
                {field.label}
                {#if field.required}
                  <span class="metadata-required-dot" aria-label="required" title="Required field">•</span>
                {/if}
              </label>
            </div>

            {#if field.widget === "text"}
              <input
                id={fieldId(field.key)}
                type="text"
                value={asText(doc.frontmatter[field.key])}
                oninput={(e) => updateField(field.key, (e.target as HTMLInputElement).value)}
                placeholder={field.required ? "Required" : "Empty"}
                class="metadata-input"
              />
            {:else if field.widget === "textarea"}
              <textarea
                id={fieldId(field.key)}
                value={asText(doc.frontmatter[field.key])}
                oninput={(e) => updateField(field.key, (e.target as HTMLTextAreaElement).value)}
                rows="3"
                placeholder="Add a description…"
                class="metadata-input metadata-textarea"
              ></textarea>
            {:else if field.widget === "datetime"}
              <input
                id={fieldId(field.key)}
                type="date"
                value={asDateInputValue(doc.frontmatter[field.key])}
                oninput={(e) => updateField(field.key, (e.target as HTMLInputElement).value)}
                class="metadata-input metadata-input-date"
              />
            {:else if field.widget === "path"}
              <input
                id={fieldId(field.key)}
                type="text"
                value={asText(doc.frontmatter[field.key])}
                oninput={(e) => updateField(field.key, (e.target as HTMLInputElement).value)}
                placeholder="/assets/images/cover.png"
                class="metadata-input metadata-input-mono"
              />
              {#if isImageField(field)}
                <div class="metadata-image-preview" data-state={thumbnails[field.key] ? "ready" : "empty"}>
                  {#if thumbnails[field.key]}
                    <img src={thumbnails[field.key]} alt="" />
                  {:else}
                    <div class="metadata-image-preview-empty">
                      <svg width="20" height="20" viewBox="0 0 20 20" fill="none" aria-hidden="true">
                        <rect x="2" y="3" width="16" height="14" rx="1.5" stroke="currentColor" stroke-width="1.1" opacity="0.4"/>
                        <circle cx="7" cy="8" r="1.4" stroke="currentColor" stroke-width="1" opacity="0.5"/>
                        <path d="M3 14l4-4 3 3 3-3 4 4" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" opacity="0.5"/>
                      </svg>
                      <span>No preview</span>
                    </div>
                  {/if}
                </div>
              {/if}
            {:else if field.widget === "checkbox"}
              <label class="metadata-checkbox-label">
                <input
                  id={fieldId(field.key)}
                  type="checkbox"
                  checked={asBool(doc.frontmatter[field.key])}
                  onchange={(e) => updateField(field.key, (e.target as HTMLInputElement).checked)}
                  class="metadata-checkbox"
                />
                <span class="metadata-checkbox-track" aria-hidden="true">
                  <span class="metadata-checkbox-thumb"></span>
                </span>
                <span class="metadata-checkbox-text">
                  {asBool(doc.frontmatter[field.key]) ? "Yes" : "No"}
                </span>
              </label>
            {:else if field.widget === "string-array"}
              <div class="metadata-chips">
                {#each asStringArray(doc.frontmatter[field.key]) as item, i}
                  <div class="metadata-chip" data-array-key={field.key}>
                    <input
                      type="text"
                      value={item}
                      oninput={(e) => updateArrayItem(field.key, i, (e.target as HTMLInputElement).value)}
                      onkeydown={(e) => handleArrayKey(e, field.key, i)}
                      placeholder={i === 0 ? "Add item…" : ""}
                      class="metadata-chip-input"
                    />
                    <button
                      onclick={() => removeArrayItem(field.key, i)}
                      class="metadata-chip-remove"
                      aria-label="Remove"
                      tabindex="-1"
                    >
                      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
                        <path d="M2 2l6 6M8 2l-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                      </svg>
                    </button>
                  </div>
                {/each}
                <button
                  onclick={() => addArrayItem(field.key)}
                  class="metadata-chip-add"
                  type="button"
                >
                  <span aria-hidden="true">+</span> Add
                </button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</aside>

<style>
  .metadata {
    display: flex;
    flex-direction: column;
    background: var(--color-chrome);
    transition: width 0.18s ease, opacity 0.18s ease, transform 0.18s ease;
    overflow: hidden;
    border-left: 1px solid var(--color-rule);
    position: relative;
    flex-shrink: 0;
  }

  .metadata-open {
    /* width is set inline by the panel-widths store */
    opacity: 1;
    transform: translateX(0);
  }

  .metadata-closed {
    width: 0;
    opacity: 0;
    transform: translateX(8px);
    pointer-events: none;
  }

  /* ===== Header ===== */
  .metadata-header {
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--color-rule);
    background: var(--color-chrome);
  }

  .metadata-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .metadata-title {
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-muted);
    margin: 0;
  }

  .metadata-dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-accent);
    box-shadow: 0 0 0 2px rgba(109, 40, 217, 0.18);
  }

  .metadata-context {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .metadata-context-collection {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
  }

  .metadata-context-collection-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-accent);
    box-shadow: 0 0 0 2px rgba(109, 40, 217, 0.18);
  }

  .metadata-context-collection-name {
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 600;
    color: var(--color-ink);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .metadata-context-file {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .metadata-context-file-name {
    font-family: var(--font-editor);
    font-size: 16px;
    font-weight: 500;
    color: var(--color-ink);
    line-height: 1.3;
    word-break: break-word;
  }

  .metadata-context-file-dir {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-muted);
    word-break: break-all;
  }

  /* ===== Scrollable body ===== */
  .metadata-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px 24px;
  }

  /* ===== Empty state ===== */
  .metadata-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 60px 24px;
    color: var(--color-muted);
  }

  .metadata-empty-illust {
    color: var(--color-muted);
    margin-bottom: 18px;
    opacity: 0.6;
  }

  .metadata-empty-title {
    font-family: var(--font-editor);
    font-size: 16px;
    font-weight: 500;
    color: var(--color-ink);
    margin: 0 0 4px;
  }

  .metadata-empty-hint {
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--color-muted);
    margin: 0;
    line-height: 1.5;
    max-width: 200px;
  }

  /* ===== Body (form fields) ===== */
  .metadata-body {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .metadata-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .metadata-field-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .metadata-field-typechip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 16px;
    padding: 0 5px;
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    border: 1px solid;
    border-radius: 3px;
    flex-shrink: 0;
    text-transform: uppercase;
  }

  .metadata-label {
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    color: var(--color-ink);
    letter-spacing: -0.005em;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .metadata-required-dot {
    color: var(--color-accent);
    font-size: 16px;
    line-height: 0;
    margin-left: 2px;
  }

  /* ===== Inputs (unified) ===== */
  .metadata-input {
    width: 100%;
    padding: 8px 11px;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--color-ink);
    background: rgba(255, 255, 255, 0.55);
    border: 1px solid transparent;
    border-radius: 6px;
    outline: none;
    transition: background 0.12s, border-color 0.12s, box-shadow 0.12s;
  }

  .metadata-input::placeholder {
    color: var(--color-muted);
    font-style: italic;
    opacity: 0.65;
  }

  .metadata-input:hover {
    background: rgba(255, 255, 255, 0.75);
    border-color: var(--color-rule);
  }

  .metadata-input:focus {
    background: white;
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px rgba(109, 40, 217, 0.12);
  }

  .metadata-input-mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  /* ===== Image field thumbnail ===== */
  .metadata-image-preview {
    margin-top: 6px;
    width: 100%;
    height: 88px;
    background: var(--color-chrome);
    border: 1px solid var(--color-rule);
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.15s;
  }

  .metadata-image-preview[data-state="ready"] {
    border-color: var(--color-rule-strong);
    background: white;
  }

  .metadata-image-preview img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    display: block;
  }

  .metadata-image-preview-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--color-muted);
    font-family: var(--font-ui);
    font-size: 10px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .metadata-input-date {
    font-variant-numeric: tabular-nums;
  }

  .metadata-textarea {
    resize: none;
    line-height: 1.55;
    min-height: 70px;
  }

  /* ===== Checkbox (iOS-style switch) ===== */
  .metadata-checkbox-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    padding: 4px 0;
  }

  .metadata-checkbox {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .metadata-checkbox-track {
    width: 32px;
    height: 18px;
    background: var(--color-rule-strong);
    border-radius: 9px;
    position: relative;
    transition: background 0.18s;
    flex-shrink: 0;
  }

  .metadata-checkbox-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: white;
    border-radius: 50%;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
    transition: transform 0.18s;
  }

  .metadata-checkbox:checked + .metadata-checkbox-track {
    background: var(--color-accent);
  }

  .metadata-checkbox:checked + .metadata-checkbox-track .metadata-checkbox-thumb {
    transform: translateX(14px);
  }

  .metadata-checkbox-text {
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--color-ink-soft);
  }

  /* ===== Chips (string array) ===== */
  .metadata-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    align-items: center;
  }

  .metadata-chip {
    display: inline-flex;
    align-items: center;
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 14px;
    padding: 0 4px 0 10px;
    height: 26px;
    transition: border-color 0.12s, box-shadow 0.12s;
  }

  .metadata-chip:focus-within {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 2px rgba(109, 40, 217, 0.12);
  }

  .metadata-chip-input {
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--color-ink);
    background: transparent;
    border: none;
    outline: none;
    width: auto;
    min-width: 60px;
    max-width: 180px;
    padding: 0;
  }

  .metadata-chip-input::placeholder {
    color: var(--color-muted);
    font-style: italic;
  }

  .metadata-chip-remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    color: var(--color-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    border-radius: 50%;
    transition: color 0.12s, background 0.12s;
    flex-shrink: 0;
    margin-left: 4px;
  }

  .metadata-chip-remove:hover {
    color: var(--color-danger);
    background: rgba(220, 38, 38, 0.1);
  }

  .metadata-chip-add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 10px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    color: var(--color-muted);
    background: transparent;
    border: 1px dashed var(--color-rule-strong);
    border-radius: 14px;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s, background 0.12s;
  }

  .metadata-chip-add:hover {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
  }
</style>
