<script lang="ts">
  /**
   * A small formatting toolbar for the writing surface.
   * Each button wraps the current selection (or inserts a placeholder
   * at the caret) with markdown syntax.
   *
   * It receives `insertAtCaret` from the WritingSurface. The actual
   * selection logic lives in the contenteditable host.
   */

  let {
    insertAtCaret,
    onImageClick,
  }: {
    insertAtCaret: (text: string, options?: { caretOffset?: number }) => void;
    /** When provided, renders an "Insert image" button at the end of the
     *  toolbar. The parent (EditorPanel) is responsible for the picker
     *  and the actual `insertAtCaret` call with the relative path. */
    onImageClick?: () => void;
  } = $props();

  type Action = {
    id: string;
    label: string;
    title: string;
    /**
     * Run with the current selection text (empty if no selection).
     * Returns the text to insert at caret and where the caret should
     * end up inside that text. Negative `caretOffset` counts from the
     * end (-1 = last position, -2 = second to last, etc.).
     *
     * For prefix-style actions (H1, list, quote…), we DON'T insert a
     * placeholder word when there's no selection — only the prefix,
     * caret right after it. The user types the content directly.
     * For inline wrappers (bold, italic, link…) we DO keep a small
     * placeholder so the user can see what to type, with the caret
     * inside the wrapping punctuation.
     */
    run: (sel: string) => { text: string; caretOffset: number };
  };

  /**
   * Triple backtick. The literal "```" is awkward to write in
   * source because backticks are template-literal delimiters, so we
   * spell it out with unicode escapes to keep the code readable.
   */
  const CODE_FENCE = "```";

  const ACTIONS: Action[] = [
    // Headings — no selection: just the prefix, caret after it.
    {
      id: "h1",
      label: "H1",
      title: "Heading 1",
      run: (sel) => sel
        ? { text: `# ${sel}`, caretOffset: -1 }
        : { text: "# ", caretOffset: 2 },
    },
    {
      id: "h2",
      label: "H2",
      title: "Heading 2",
      run: (sel) => sel
        ? { text: `## ${sel}`, caretOffset: -1 }
        : { text: "## ", caretOffset: 3 },
    },
    // Inline wrappers — caret between the markers, placeholder visible.
    {
      id: "bold",
      label: "B",
      title: "Bold (Ctrl+B)",
      run: (sel) => sel
        ? { text: `**${sel}**`, caretOffset: -2 }
        : { text: "****", caretOffset: 2 },
    },
    {
      id: "italic",
      label: "I",
      title: "Italic (Ctrl+I)",
      run: (sel) => sel
        ? { text: `*${sel}*`, caretOffset: -1 }
        : { text: "**", caretOffset: 1 },
    },
    {
      id: "strike",
      label: "S",
      title: "Strikethrough",
      run: (sel) => sel
        ? { text: `~~${sel}~~`, caretOffset: -2 }
        : { text: "~~~~", caretOffset: 2 },
    },
    // Block prefixes — no selection: just the prefix, caret after it.
    {
      id: "ul",
      label: "•",
      title: "Bullet list",
      run: (sel) => sel
        ? { text: `- ${sel}`, caretOffset: -1 }
        : { text: "- ", caretOffset: 2 },
    },
    {
      id: "ol",
      label: "1.",
      title: "Numbered list",
      run: (sel) => sel
        ? { text: `1. ${sel}`, caretOffset: -1 }
        : { text: "1. ", caretOffset: 3 },
    },
    {
      id: "quote",
      label: "❝",
      title: "Quote",
      run: (sel) => sel
        ? { text: `> ${sel}`, caretOffset: -1 }
        : { text: "> ", caretOffset: 2 },
    },
    // Code block — triple-backtick fenced.
    // - no selection: insert "```\n\n```" with caret on the empty
    //   line in the middle, ready to type code.
    // - with selection: wrap it in fences, caret stays at the end
    //   of the selected text so the user keeps editing inside the
    //   block.
    {
      id: "code",
      label: "</>",
      title: "Code block (```)",
      run: (sel) => sel
        ? { text: `${CODE_FENCE}\n${sel}\n${CODE_FENCE}`, caretOffset: -1 }
        : { text: `${CODE_FENCE}\n\n${CODE_FENCE}`, caretOffset: 4 },
    },
    // Link — caret on the "text" part with "url" as placeholder.
    {
      id: "link",
      label: "↗",
      title: "Link",
      run: (sel) => sel
        ? { text: `[${sel}](url)`, caretOffset: -5 }
        : { text: "[text](url)", caretOffset: 1 },
    },
  ];

  function applyAction(action: Action) {
    const sel = window.getSelection()?.toString() ?? "";
    const { text, caretOffset } = action.run(sel);
    insertAtCaret(text, { caretOffset });
  }

  /**
   * Apply one of the formatting actions by id.
   *
   * Exposed so the writing surface can bind Ctrl+B / Ctrl+I to the
   * very same markdown transformation (and caret placement) the
   * buttons use — the shortcut and the toolbar can never drift apart.
   */
  export function applyActionById(id: string): boolean {
    const action = ACTIONS.find((a) => a.id === id);
    if (!action) return false;
    applyAction(action);
    return true;
  }
</script>

<div class="editor-toolbar" role="toolbar" aria-label="Formatting">
  {#each ACTIONS as action (action.id)}
    <button
      type="button"
      class="toolbar-btn"
      title={action.title}
      aria-label={action.title}
      onclick={() => applyAction(action)}
    >
      {action.label}
    </button>
  {/each}

  {#if onImageClick}
    <span class="toolbar-divider" aria-hidden="true"></span>
    <button
      type="button"
      class="toolbar-btn toolbar-btn-icon"
      title="Insert image (Ctrl+Shift+I)"
      aria-label="Insert image"
      onclick={onImageClick}
    >
      <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" stroke="currentColor" stroke-width="1.4"/>
        <circle cx="5.5" cy="6.5" r="1.2" stroke="currentColor" stroke-width="1.2"/>
        <path d="M2 11l3.2-3.2a.5.5 0 0 1 .7 0L14 13" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M9 9.5l1.8-1.8a.5.5 0 0 1 .7 0L14 11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
  {/if}
</div>

<style>
  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 6px 14px;
    background: var(--color-chrome);
    border-bottom: 1px solid var(--color-rule);
    border-top: 1px solid var(--color-rule);
  }

  .toolbar-btn {
    min-width: 34px;
    height: 32px;
    padding: 0 10px;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--color-ink-soft);
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 6px;
    cursor: pointer;
    transition: color 0.12s, background 0.12s, border-color 0.12s, transform 0.08s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    user-select: none;
  }

  .toolbar-btn-icon {
    min-width: 34px;
    padding: 0;
  }

  .toolbar-divider {
    width: 1px;
    height: 18px;
    background: var(--color-rule);
    margin: 0 4px;
    flex-shrink: 0;
  }

  .toolbar-btn:hover {
    color: var(--color-accent);
    background: white;
    border-color: var(--color-accent);
  }

  .toolbar-btn:active {
    background: var(--color-accent-soft);
    transform: translateY(1px);
  }
</style>
