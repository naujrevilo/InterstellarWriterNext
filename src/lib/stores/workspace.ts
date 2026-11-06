import { writable, get } from "svelte/store";
import type {
  WorkspaceInfo,
  WorkspaceKind,
  DocumentRef,
  CollectionInfo,
} from "../types/workspace";
import * as api from "../api/tauri";
import { notifications } from "./notifications";
import { documentStore } from "./document";

/** Hard timeout for a single scan. If the Rust side hangs for any reason,
 *  we surface a clear error to the user instead of leaving them stuck on
 *  "Opening…" forever. The scan itself is capped at 5k files / 16 levels
 *  deep on the Rust side, so this is just a backstop. */
const SCAN_TIMEOUT_MS = 8_000;

interface WorkspaceState {
  root: string;
  kind: WorkspaceKind;
  collections: CollectionInfo[];
  files: DocumentRef[];
  selectedFile: DocumentRef | null;
  loading: boolean;
  error: string | null;
}

function createWorkspaceStore() {
  const { subscribe, set, update } = writable<WorkspaceState>({
    root: "",
    kind: "generic-markdown",
    collections: [],
    files: [],
    selectedFile: null,
    loading: false,
    error: null,
  });

  // A single in-flight workspace transition at a time. If the user clicks
  // "Cancel" or opens a different folder mid-scan, we still need the loading
  // state to recover and an older result must not replace the newer one.
  let inflightAbort: { cancelled: boolean } | null = null;

  function beginTransition(): { cancelled: boolean } {
    if (inflightAbort) inflightAbort.cancelled = true;
    const token = { cancelled: false };
    inflightAbort = token;
    return token;
  }

  function isCurrentTransition(token: { cancelled: boolean }): boolean {
    return !token.cancelled && inflightAbort === token;
  }

  function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`${label} timed out after ${ms}ms`)),
        ms
      );
      promise.then(
        (v) => {
          clearTimeout(timer);
          resolve(v);
        },
        (e) => {
          clearTimeout(timer);
          reject(e);
        }
      );
    });
  }

  async function loadWorkspace(root: string) {
    console.log("[workspace] loadWorkspace called with:", root);
    const token = beginTransition();

    // Guard before the backend command switches its workspace root. The
    // document must also be cleared before that command can make the new
    // workspace active.
    const canDiscard = await documentStore.confirmDiscardUnsavedChanges();
    if (!canDiscard) {
      if (inflightAbort === token) {
        inflightAbort = null;
        update((s) => ({ ...s, loading: false }));
      }
      return;
    }

    if (!isCurrentTransition(token)) return;

    documentStore.reset();
    update((s) => ({
      ...s,
      selectedFile: null,
      loading: true,
      error: null,
    }));

    try {
      console.log("[workspace] calling scanWorkspace...");
      const info = await withTimeout(
        api.openWorkspace(root),
        SCAN_TIMEOUT_MS,
        "scan_workspace"
      );

      // If the user cancelled while we were scanning, or a newer transition
      // started, drop the result silently.
      if (!isCurrentTransition(token)) {
        console.log("[workspace] scan finished but was cancelled — discarding");
        return;
      }

      console.log("[workspace] scan result:", {
        files: info.files.length,
        collections: info.collections.length,
        kind: info.kind,
      });

      set({
        root: info.root,
        kind: info.kind,
        collections: info.collections,
        files: info.files,
        selectedFile: null,
        loading: false,
        error: null,
      });

      notifications.success(`Workspace opened — ${info.files.length} files`);
    } catch (e: any) {
      const msg = e?.message ?? String(e);
      console.error("[workspace] loadWorkspace error:", msg);
      // Ignore errors from a transition that a newer request has replaced.
      if (inflightAbort === token) {
        update((s) => ({ ...s, loading: false, error: msg }));
        notifications.error(msg);
      }
    } finally {
      if (inflightAbort === token) inflightAbort = null;
    }
  }

  return {
    subscribe,

    async openWorkspace() {
      console.log("[workspace] openWorkspace (dialog) called");
      try {
        const { root } = await api.selectWorkspace();
        console.log("[workspace] dialog returned:", root);
        await loadWorkspace(root);
      } catch (e: any) {
        const msg = e?.message ?? String(e);
        console.error("[workspace] openWorkspace error:", msg);
        // Don't notify on cancel
        if (!msg.includes("No folder selected")) {
          update((s) => ({ ...s, loading: false, error: msg }));
          notifications.error(msg);
        } else {
          update((s) => ({ ...s, loading: false }));
        }
      }
    },

    async openWorkspaceAtPath(path: string) {
      console.log("[workspace] openWorkspaceAtPath:", path);
      await loadWorkspace(path);
    },

    async selectFile(file: DocumentRef): Promise<boolean> {
      console.log("[workspace] selectFile:", file.relativePath);
      const current = get({ subscribe: workspaceStore.subscribe });

      // Re-selecting the open document must neither prompt nor reload it.
      if (current.selectedFile?.relativePath === file.relativePath) {
        return true;
      }

      if (!(await documentStore.confirmDiscardUnsavedChanges())) {
        return false;
      }

      // Clear the old document before the selection change triggers a load.
      // reset() also invalidates any stale in-flight document request.
      documentStore.reset();
      update((s) => ({ ...s, selectedFile: file }));
      return true;
    },

    async createDocument(input: {
      title: string;
      filename: string;
      collection: string | null;
      directory: string;
    }): Promise<boolean> {
      const current = get({ subscribe });
      if (!current.root || current.loading) return false;
      if (!(await documentStore.confirmDiscardUnsavedChanges())) return false;
      await documentStore.waitForPendingSave();

      const latest = get({ subscribe });
      if (latest.root !== current.root || latest.selectedFile !== current.selectedFile) {
        notifications.error("The workspace or selected document changed. Please try again.");
        return false;
      }

      const file = await api.createDocument(input);
      if (get({ subscribe }).root !== current.root) {
        notifications.error(`Created ${file.relativePath}, but the workspace changed before it could be opened.`);
        return true;
      }
      documentStore.reset();
      update((s) => ({
        ...s,
        files: [...s.files.filter((item) => item.relativePath !== file.relativePath), file]
          .sort((a, b) => a.relativePath.localeCompare(b.relativePath)),
        collections: s.collections.map((col) =>
          col.name === file.collection && !s.files.some((item) => item.relativePath === file.relativePath)
            ? { ...col, documentCount: col.documentCount + 1 }
            : col
        ),
        selectedFile: file,
      }));
      return true;
    },

    /**
     * Re-scan the current workspace silently. Used after a save to
     * pick up changes in derived metadata (e.g. `isDraft` flag, which
     * is peeked at scan time and won't update when the user toggles
     * the boolean in the metadata panel and saves).
     *
     * Preserves the currently selected file by path; updates its
     * `isDraft`/`titleHint` in-place so the sidebar reflects the new
     * state without forcing a re-select.
     */
    async refresh() {
      const current = get({ subscribe: workspaceStore.subscribe });
      if (!current.root) return;
      const selectedPath = current.selectedFile?.relativePath ?? null;
      try {
        const info = await withTimeout(
          api.openWorkspace(current.root),
          SCAN_TIMEOUT_MS,
          "scan_workspace (refresh)"
        );
        // Merge: keep selection; update `isDraft`/`titleHint` for the
        // selected file from the fresh scan.
        const files = selectedPath
          ? info.files.map((f) =>
              f.relativePath === selectedPath
                ? { ...f, isDraft: f.isDraft, titleHint: f.titleHint }
                : f
            )
          : info.files;
        update((s) => ({
          ...s,
          files,
          collections: info.collections,
          kind: info.kind,
        }));
      } catch (e: any) {
        // Silent on refresh — the next save or action will surface
        // real errors.
        console.warn("[workspace] refresh failed:", e?.message ?? e);
      }
    },

    async trashFile(relativePath: string) {
      await api.moveDocumentToTrash(relativePath);
      const current = get({ subscribe: workspaceStore.subscribe });
      if (current.selectedFile?.relativePath === relativePath) {
        documentStore.reset();
        update((s) => ({ ...s, selectedFile: null }));
      }
      await workspaceStore.refresh();
    },

    /** Cancel the current in-flight scan, if any, and reset loading state. */
    cancel() {
      console.log("[workspace] cancel");
      if (inflightAbort) inflightAbort.cancelled = true;
      inflightAbort = null;
      update((s) => ({ ...s, loading: false }));
    },

    async reset(): Promise<boolean> {
      console.log("[workspace] reset");
      const token = beginTransition();

      if (!(await documentStore.confirmDiscardUnsavedChanges())) {
        if (inflightAbort === token) {
          inflightAbort = null;
          update((s) => ({ ...s, loading: false }));
        }
        return false;
      }

      if (!isCurrentTransition(token)) return false;

      documentStore.reset();
      set({
        root: "",
        kind: "generic-markdown",
        collections: [],
        files: [],
        selectedFile: null,
        loading: false,
        error: null,
      });
      if (inflightAbort === token) inflightAbort = null;
      return true;
    },
  };
}

export const workspaceStore = createWorkspaceStore();
