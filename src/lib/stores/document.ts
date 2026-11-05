import { writable, derived, get } from "svelte/store";
import type { DocumentKind } from "../types/workspace";
import type { DocumentState } from "../types/document";
import { createDocumentState } from "../types/document";
import * as api from "../api/tauri";
import { notifications } from "./notifications";

/** Maximum number of past states we keep in the undo stack. */
const HISTORY_LIMIT = 200;

function createDocumentStore() {
  const { subscribe, set, update } = writable<DocumentState>(
    createDocumentState()
  );
  let loadRequest = 0;

  return {
    subscribe,

    async load(relativePath: string, kind: DocumentKind) {
      const request = ++loadRequest;
      try {
        const { raw } = await api.readDocument(relativePath);
        const parsed = await api.parseDocument(raw, kind);
        // A slower prior request must never replace the document the user
        // selected after it started loading.
        if (request !== loadRequest) return;
        set({
          relativePath,
          kind,
          frontmatter: parsed.frontmatter,
          body: parsed.body,
          raw: parsed.raw,
          dirty: false,
          // Reset history on file load — undo never crosses file
          // boundaries (a basic safety rule).
          history: { past: [], future: [] },
        });
      } catch (e: any) {
        if (request !== loadRequest) return;
        notifications.error(`Failed to load: ${e?.message ?? e}`);
      }
    },

    /**
     * Confirm whether the current document may be discarded. This method
     * never mutates document state: callers can safely keep the current
     * document when this returns false.
     */
    async confirmDiscardUnsavedChanges(): Promise<boolean> {
      const current = get({ subscribe });
      if (!current.dirty) return true;

      try {
        return await api.confirmDiscardChanges();
      } catch (e: any) {
        console.error("[document] discard confirmation failed:", e);
        notifications.error(
          "Could not confirm discarding changes. Your changes were kept."
        );
        return false;
      }
    },

    updateBody(body: string) {
      update((s) => {
        // No-op when the new body is identical — avoids polluting
        // history with "self" changes that happen when the parent
        // re-renders and sends the same value back.
        if (s.body === body) return s;

        return {
          ...s,
          body,
          dirty: true,
          history: pushHistory(s.history, s.body),
        };
      });
    },

    updateFrontmatter(frontmatter: Record<string, unknown>) {
      update((s) => ({
        ...s,
        frontmatter,
        dirty: true,
        // Frontmatter changes are NOT added to the body history — undo
        // only rewinds the markdown content. A future iteration can
        // extend this to also track frontmatter snapshots.
      }));
    },

    /** Undo the last body change. No-op if there's nothing to undo. */
    undo() {
      let didUndo = false;
      update((s) => {
        const last = s.history.past[s.history.past.length - 1];
        if (last === undefined) return s;
        didUndo = true;
        return {
          ...s,
          body: last,
          dirty: true,
          history: {
            past: s.history.past.slice(0, -1),
            future: [...s.history.future, s.body],
          },
        };
      });
      return didUndo;
    },

    /** Redo the next body change. No-op if there's nothing to redo. */
    redo() {
      let didRedo = false;
      update((s) => {
        const next = s.history.future[s.history.future.length - 1];
        if (next === undefined) return s;
        didRedo = true;
        return {
          ...s,
          body: next,
          dirty: true,
          history: {
            past: [...s.history.past, s.body],
            future: s.history.future.slice(0, -1),
          },
        };
      });
      return didRedo;
    },

    async save() {
      const current = get({ subscribe });

      if (!current.relativePath || !current.dirty) return;

      try {
        const { raw } = await api.serializeDocument(
          current.frontmatter,
          current.body,
          current.kind
        );
        await api.writeDocument(current.relativePath, raw);
        update((s) => ({ ...s, raw, dirty: false }));
        notifications.success("Saved");
      } catch (e: any) {
        notifications.error(`Failed to save: ${e?.message ?? e}`);
      }
    },

    /** Clear document state after the caller has authorized any discard. */
    reset() {
      loadRequest += 1;
      set(createDocumentState());
    },
  };
}

interface History {
  past: string[];
  future: string[];
}

/**
 * Push the current body onto the past stack, drop the future stack.
 * Caps the past stack at HISTORY_LIMIT entries.
 */
function pushHistory(history: History, previousBody: string): History {
  return {
    past: [...history.past, previousBody].slice(-HISTORY_LIMIT),
    future: [],
  };
}

export const documentStore = createDocumentStore();

export const isDirty = derived(documentStore, ($doc) => $doc.dirty);
export const canUndo = derived(documentStore, ($doc) => $doc.history.past.length > 0);
export const canRedo = derived(documentStore, ($doc) => $doc.history.future.length > 0);
