import type {
  WorkspaceInfo,
  ParsedDocument,
  DocumentKind,
} from "./workspace";

export type SyncOutcome =
  | { kind: "synced"; message: string }
  | { kind: "no_changes" }
  | { kind: "auth_failed"; reason: string }
  | { kind: "scope_missing"; reason: string }
  | { kind: "conflict"; reason: string }
  | { kind: "failed"; reason: string };

/**
 * IPC contract for the Tauri commands registered in `src-tauri/src/lib.rs`.
 *
 * The workspace root is established by `open_workspace` and then held in
 * backend state. Document commands receive paths relative to that root.
 * Git Sync's `repoPath` is not an alternate root: the backend accepts it
 * only when it matches the canonical workspace root exactly.
 */
export interface TauriCommands {
  open_workspace: (input: { root: string }) => Promise<WorkspaceInfo>;
  detect_workspace_kind: () => Promise<{
    kind: "astro" | "generic-markdown";
    signals: string[];
  }>;
  read_document: (input: {
    relativePath: string;
  }) => Promise<{ raw: string }>;
  write_document: (input: {
    relativePath: string;
    raw: string;
  }) => Promise<{ ok: true }>;
  parse_document: (input: {
    raw: string;
    kind: DocumentKind;
  }) => Promise<ParsedDocument>;
  serialize_document: (input: {
    frontmatter: Record<string, unknown>;
    body: string;
    kind: DocumentKind;
  }) => Promise<{ raw: string }>;
  resolve_image: (input: {
    fromFile: string;
    imagePath: string;
  }) => Promise<{ dataUrl: string; mime: string; size: number }>;
  save_github_token: (input: { token: string }) => Promise<void>;
  clear_github_token: () => Promise<void>;
  github_token_configured: () => Promise<boolean>;
  sync_to_github: (input: {
    repoPath: string;
    commitMessage: string;
  }) => Promise<SyncOutcome>;
}
