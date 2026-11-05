import { invoke } from "@tauri-apps/api/core";
import type {
  WorkspaceInfo,
  ParsedDocument,
  DocumentKind,
} from "../types/workspace";

function isTauri(): boolean {
  const has = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  console.log("[api] isTauri:", has);
  return has;
}

export async function selectWorkspace(): Promise<{ root: string }> {
  console.log("[api] selectWorkspace called, isTauri:", isTauri());

  if (!isTauri()) {
    throw new Error("Not running in Tauri context. Use the native window, not the browser.");
  }

  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    console.log("[api] dialog plugin loaded, opening...");
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Workspace Folder",
    });
    console.log("[api] dialog result:", selected);

    if (!selected) {
      throw new Error("No folder selected");
    }

    return { root: selected as string };
  } catch (e: any) {
    console.error("[api] selectWorkspace error:", e);
    throw e;
  }
}

/**
 * Ask the user to confirm that unsaved document changes may be discarded.
 * A rejected promise or a negative answer must be treated as "do not
 * discard" by the caller.
 */
export async function confirmDiscardChanges(): Promise<boolean> {
  if (!isTauri()) {
    throw new Error("Not running in Tauri context. Use the native window, not the browser.");
  }

  const { confirm } = await import("@tauri-apps/plugin-dialog");
  return confirm("You have unsaved changes. Discard them and continue?", {
    title: "Unsaved changes",
    kind: "warning",
    okLabel: "Discard",
    cancelLabel: "Keep editing",
  });
}

export async function openWorkspace(root: string): Promise<WorkspaceInfo> {
  console.log("[api] openWorkspace:", root);
  try {
    const result = await invoke<WorkspaceInfo>("open_workspace", { root });
    console.log("[api] openWorkspace success:", result.files.length, "files");
    return result;
  } catch (e: any) {
    console.error("[api] scanWorkspace error:", e);
    throw e;
  }
}

export async function readDocument(relativePath: string): Promise<{ raw: string }> {
  return invoke("read_document", { relativePath });
}

export async function parseDocument(
  raw: string,
  kind: DocumentKind
): Promise<ParsedDocument> {
  return invoke("parse_document", { raw, kind });
}

export async function serializeDocument(
  frontmatter: Record<string, unknown>,
  body: string,
  kind: DocumentKind
): Promise<{ raw: string }> {
  return invoke("serialize_document", { frontmatter, body, kind });
}

export async function writeDocument(
  relativePath: string,
  raw: string,
  expectedRaw?: string
): Promise<{ ok: true }> {
  return invoke("write_document", { relativePath, raw, expectedRaw });
}

export async function confirmOverwriteExternalChanges(): Promise<boolean> {
  const { confirm } = await import("@tauri-apps/plugin-dialog");
  return confirm(
    "This file changed on disk after you opened it. Overwrite it with your version? The current file is backed up first.",
    {
      title: "File changed on disk",
      kind: "warning",
      okLabel: "Overwrite",
      cancelLabel: "Cancel",
    }
  );
}

/**
 * Resolve an image path (relative or absolute) to a `data:` URL.
 * The Rust side validates the path is inside the workspace before
 * reading the bytes, so this is safe to call with user input.
 */
export async function resolveImage(
  fromFile: string,
  imagePath: string
): Promise<{ dataUrl: string; mime: string; size: number }> {
  return invoke("resolve_image", { fromFile, imagePath });
}

/**
 * Outcome of a GitHub sync attempt. The Rust side maps the
 * `services::git::GitError` enum to one of these so the UI can
 * react (settings prompt on auth failure, conflict modal on
 * merge conflict, success toast, etc.).
 */
export type SyncOutcome =
  | { kind: "synced"; message: string }
  | { kind: "no_changes" }
  | { kind: "auth_failed"; reason: string }
  | { kind: "scope_missing"; reason: string }
  | { kind: "conflict"; reason: string }
  | { kind: "failed"; reason: string };

/**
 * Push the current state of the repo at `repoPath` to its `origin`
 * remote. The Rust side stages everything, commits, fast-forwards or
 * merges, and pushes in one call.
 *
 * The personal access token is not part of this call: the backend reads
 * it from the operating-system credential store and uses it internally.
 */
export async function syncToGithub(
  repoPath: string,
  commitMessage: string
): Promise<SyncOutcome> {
  return invoke<SyncOutcome>("sync_to_github", {
    repoPath,
    commitMessage,
  });
}

/**
 * Store the GitHub personal access token in the operating-system
 * credential store (Windows Credential Manager, macOS Keychain, Secret
 * Service on Linux).
 *
 * The token leaves the frontend only on its way into the credential
 * store: it is never persisted in localStorage and never read back.
 */
export async function saveGithubToken(token: string): Promise<void> {
  await invoke("save_github_token", { token });
}

/** Remove the GitHub token from the operating-system credential store. */
export async function clearGithubToken(): Promise<void> {
  await invoke("clear_github_token");
}

/**
 * Whether a GitHub token is stored in the credential store.
 *
 * The backend answers with a boolean only — the secret never leaves it.
 */
export async function githubTokenConfigured(): Promise<boolean> {
  return invoke<boolean>("github_token_configured");
}
