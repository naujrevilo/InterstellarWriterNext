<script lang="ts">
  import { preferencesStore } from "../../stores/preferences";
  import EditorToolbar from "./EditorToolbar.svelte";
  import { documentStore } from "../../stores/document";

  /**
   * Why `contenteditable` instead of `<textarea>`:
   *
   * - WebView2 (Tauri on Windows) has known quirks with text selection in
   *   `<textarea>` — drag selection sometimes doesn't engage, especially
   *   when the textarea is in a flex column with custom padding.
   * - `contenteditable` divs are more robust for selection across all
   *   browser engines.
   * - `contenteditable="plaintext-only"` keeps plain-text behaviour
   *   (no rich-text formatting) while behaving like a real text editor.
   */

  let {
    value = "",
    oninput,
    onkeydown,
    onImageClick,
  }: {
    value?: string;
    oninput?: (value: string) => void;
    onkeydown?: (e: KeyboardEvent) => void;
    /** Triggered by the "Insert image" toolbar button. The parent is
     *  responsible for picking the file and calling our `insertAtCaret`
     *  via the bound `bind:this` ref. */
    onImageClick?: () => void;
  } = $props();

  let { editorFont } = $derived($preferencesStore);

  let editorEl: HTMLDivElement | null = $state(null);
  let focused = $state(false);
  let isComposing = $state(false);
  /**
   * The formatting toolbar, bound so the keyboard shortcuts can reuse
   * its markdown actions instead of duplicating them here.
   */
  let toolbarRef: { applyActionById: (id: string) => boolean } | null = $state(null);

  // Approximate word count for the subtle status indicator.
  let wordCount = $derived(value.trim() ? value.trim().split(/\s+/).length : 0);
  let charCount = $derived(value.length);

  // External value → element. We only sync when the incoming value
  // differs from the current element content, to avoid clobbering the
  // caret position while the user is typing.
  $effect(() => {
    if (!editorEl) return;
    if (isComposing) return; // don't disturb an IME composition
    if (editorEl.innerText === value) return;
    // Save and restore selection when programmatically updating.
    const wasFocused = document.activeElement === editorEl;
    let savedRange: Range | null = null;
    if (wasFocused) {
      const sel = window.getSelection();
      if (sel && sel.rangeCount > 0) {
        savedRange = sel.getRangeAt(0);
      }
    }
    editorEl.innerText = value;
    if (wasFocused && savedRange) {
      editorEl.focus();
      const sel = window.getSelection();
      if (sel) {
        sel.removeAllRanges();
        try {
          sel.addRange(savedRange);
        } catch {
          // range might be invalid; ignore
        }
      }
    }
  });

  function handleInput() {
    if (!editorEl) return;
    if (isComposing) return;
    oninput?.(editorEl.innerText);
  }

  /**
   * Intercept undo/redo shortcuts. We call the document store's undo/redo,
   * which restores the previous value via state — the $effect above
   * syncs the contenteditable to match. We always preventDefault on these
   * shortcuts to keep behavior consistent across browsers.
   */
  /**
   * Inspect the line the caret is currently on. Returns the prefix
   * the next list item should use (e.g. "2. " or "- ") and the
   * content after the prefix, or null if the line isn't a list item.
   *
   * Used by the Enter handler to auto-continue numbered / bulleted
   * lists. Detection rules:
   *   - numbered:  /^\d+\.\s+(.*)$/          → next is "(n+1). "
   *   - bulleted:  /^[-*•]\s+(.*)$/          → next is "<marker> "
   */
  function getCurrentListItem(): { nextPrefix: string; content: string } | null {
    if (!editorEl) return null;
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    const range = sel.getRangeAt(0);
    if (!editorEl.contains(range.endContainer)) return null;

    // Compute caret offset inside the editor's plain text.
    const preRange = document.createRange();
    preRange.setStart(editorEl, 0);
    preRange.setEnd(range.endContainer, range.endOffset);
    const caretOffset = preRange.toString().length;

    const fullText = editorEl.innerText;
    const before = fullText.substring(0, caretOffset);
    const lineStart = before.lastIndexOf("\n") + 1;
    const after = fullText.substring(caretOffset);
    const lineEndInAfter = after.indexOf("\n");
    const lineEnd = lineEndInAfter === -1 ? fullText.length : caretOffset + lineEndInAfter;
    const lineText = fullText.substring(lineStart, lineEnd);

    const numbered = /^(\d+)\.\s+(.*)$/.exec(lineText);
    if (numbered) {
      const n = parseInt(numbered[1], 10);
      return {
        nextPrefix: `${n + 1}. `,
        content: numbered[2],
      };
    }
    const bulleted = /^([-*•])\s+(.*)$/.exec(lineText);
    if (bulleted) {
      return {
        nextPrefix: `${bulleted[1]} `,
        content: bulleted[2],
      };
    }
    return null;
  }

  function handleKeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;

    // Undo: Ctrl+Z (Windows/Linux) or Cmd+Z (macOS). No shift.
    if (mod && !e.shiftKey && (e.key === "z" || e.key === "Z")) {
      e.preventDefault();
      documentStore.undo();
      return;
    }
    // Redo: Ctrl+Y or Ctrl+Shift+Z (Windows/Linux), Cmd+Shift+Z (macOS).
    if (
      (mod && !e.shiftKey && (e.key === "y" || e.key === "Y")) ||
      (mod && e.shiftKey && (e.key === "z" || e.key === "Z"))
    ) {
      e.preventDefault();
      documentStore.redo();
      return;
    }

    // Bold / Italic — Ctrl+B / Ctrl+I (Cmd on macOS).
    //
    // These keys are also bound globally to the side panels
    // (App.svelte), so we claim them here, inside the writing
    // surface: we prevent the browser default and stop the event
    // from reaching the window handler. The markdown text and the
    // caret placement come from the toolbar actions, so the shortcut
    // and the buttons behave identically.
    if (mod && !e.shiftKey && !e.altKey) {
      const key = e.key.toLowerCase();
      if (key === "b" || key === "i") {
        e.preventDefault();
        e.stopPropagation();
        toolbarRef?.applyActionById(key === "b" ? "bold" : "italic");
        return;
      }
    }

    // Enter (no modifier, no shift): auto-continue lists.
    if (e.key === "Enter" && !mod && !e.shiftKey && !e.altKey) {
      const item = getCurrentListItem();
      if (item) {
        e.preventDefault();
        if (item.content.trim() === "") {
          // Empty list item — exit the list. Insert a bare newline so
          // the user lands on a clean line and can keep typing.
          insertAtCaret("\n");
        } else {
          // Continue the list: newline + next prefix, caret after it.
          insertAtCaret("\n" + item.nextPrefix, {
            caretOffset: item.nextPrefix.length,
          });
        }
        return;
      }
    }

    onkeydown?.(e);
  }

  function handleCompositionStart() {
    isComposing = true;
  }

  function handleCompositionEnd() {
    isComposing = false;
    handleInput();
  }

  /**
   * Insert text at the current caret position (or replace the current
   * selection). The `caretOffset` controls where the caret ends up
   * inside the inserted text (negative = from the end).
   *
   * Exported so the parent (EditorPanel) can call it via `bind:this`
   * after the user picks an image from the file dialog.
   */
  export function insertAtCaret(
    text: string,
    options?: { caretOffset?: number }
  ): void {
    if (!editorEl) return;
    editorEl.focus();

    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) {
      // No selection — append to end
      editorEl.innerText = (editorEl.innerText ?? "") + text;
      handleInput();
      placeCaretAtEnd();
      return;
    }

    const range = sel.getRangeAt(0);
    if (!editorEl.contains(range.commonAncestorContainer)) {
      editorEl.innerText = (editorEl.innerText ?? "") + text;
      handleInput();
      placeCaretAtEnd();
      return;
    }

    range.deleteContents();
    const node = document.createTextNode(text);
    range.insertNode(node);

    // Place caret at desired offset within the inserted text.
    // Negative offsets count from the end (-1 = before the last char,
    // -2 = two before the end, etc.) so the toolbar can place the
    // caret right after a markdown prefix like "- " without the
    // placeholder word being glued to whatever comes next.
    const caretOffset = options?.caretOffset ?? text.length;
    const caret =
      caretOffset < 0
        ? Math.max(0, text.length + caretOffset)
        : Math.min(text.length, caretOffset);
    const newRange = document.createRange();
    newRange.setStart(node, caret);
    newRange.collapse(true);
    sel.removeAllRanges();
    sel.addRange(newRange);
    handleInput();
  }

  function placeCaretAtEnd() {
    if (!editorEl) return;
    editorEl.focus();
    const sel = window.getSelection();
    if (!sel) return;
    const range = document.createRange();
    range.selectNodeContents(editorEl);
    range.collapse(false);
    sel.removeAllRanges();
    sel.addRange(range);
  }
</script>

<div class="writing-surface" class:writing-surface-focus={focused}>
  <EditorToolbar bind:this={toolbarRef} {insertAtCaret} {onImageClick} />

  <div class="writing-page">
    <!--
      contenteditable="plaintext-only" preserves the textarea feel:
      no rich-text formatting is applied by the browser, only plain text
      and line breaks. We handle all the markdown insertion ourselves.
    -->
    <div
      bind:this={editorEl}
      role="textbox"
      aria-multiline="true"
      aria-label="Document editor"
      contenteditable="plaintext-only"
      spellcheck="true"
      tabindex="0"
      oninput={handleInput}
      onkeydown={handleKeydown}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      oncompositionstart={handleCompositionStart}
      oncompositionend={handleCompositionEnd}
      class="writing-editor"
      style="font-family: '{editorFont}', Georgia, 'Times New Roman', serif"
      data-placeholder="Start writing…"
    ></div>
  </div>

  <footer class="writing-status" aria-live="polite">
    <span>{wordCount.toLocaleString()} {wordCount === 1 ? "word" : "words"}</span>
    <span class="writing-status-sep" aria-hidden="true">·</span>
    <span>{charCount.toLocaleString()} chars</span>
  </footer>
</div>

<style>
  .writing-surface {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--color-paper);
    transition: background 0.2s ease;
    user-select: text;
  }

  .writing-surface-focus {
    background: var(--color-paper);
  }

  .writing-page {
    flex: 1;
    display: flex;
    justify-content: center;
    overflow-y: auto;
    padding: 6vh 24px 12vh;
    user-select: text;
    -webkit-user-select: text;
  }

  /*
   * The editable div itself. We rely on user-select on this element
   * for text selection. The placeholder is shown via a CSS
   * pseudo-element when the content is empty and the div is not focused.
   */
  .writing-editor {
    width: 100%;
    max-width: var(--editor-max-width);
    min-height: 100%;
    color: var(--color-ink);
    font-size: var(--text-lg);
    line-height: 1.78;
    padding: 0;
    letter-spacing: -0.005em;
    caret-color: var(--color-accent);
    user-select: text;
    -webkit-user-select: text;
    outline: none;
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  .writing-editor:empty::before {
    content: attr(data-placeholder);
    color: var(--color-muted);
    font-style: italic;
    opacity: 0.6;
    pointer-events: none;
  }

  .writing-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 24px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-muted);
    user-select: none;
  }

  .writing-status-sep {
    opacity: 0.5;
  }
</style>
