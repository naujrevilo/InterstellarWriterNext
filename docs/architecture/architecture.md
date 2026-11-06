# Architecture — Interstellar Writer Next

## Stack

- **Frontend**: Svelte 5 + Vite 6 + Tailwind CSS 3 + TypeScript
- **Backend**: Rust + Tauri v2
- **Package manager**: pnpm
- **Desktop integrations**: Tauri dialog, filesystem, Git (`git2`), and OS credential store (`keyring`)

The canonical technical specification is the root [`SPEC_TECNICO_FINAL.md`](../../SPEC_TECNICO_FINAL.md). This file describes the current runtime tree and IPC contract.

## Separation of concerns

```text
Frontend (Svelte)                    Backend (Rust/Tauri)
├─ UI components                     ├─ Filesystem operations
├─ State management                  ├─ Canonical path validation
├─ User interaction                  ├─ Document parsing/serialization
├─ Visual composition                ├─ Workspace detection and scanning
├─ Preview rendering                 ├─ Local image resolution
└─ Tauri invoke wrappers             ├─ Git commit/push
                                     └─ OS credential-store management
```

**Rule**: Frontend never touches disk directly. All filesystem operations go through Tauri commands.

## Directory structure

```text
InterstellarWriterNext/
├── src/                              # Frontend (Svelte)
│   ├── lib/
│   │   ├── api/tauri.ts              # Typed Tauri invoke wrappers
│   │   ├── components/
│   │   │   ├── editor/               # Writing surface, toolbar, fonts, Git Sync
│   │   │   ├── layout/               # Sidebar, editor panel, metadata panel
│   │   │   ├── preview/              # Preview component
│   │   │   └── shared/               # Toast notifications
│   │   ├── features/preview/         # Markdown, Mermaid, MDX, and Astro-aware rendering
│   │   ├── stores/                   # Workspace, document, UI, preferences, Git, notifications
│   │   ├── styles/                   # Tokens, typography, app styles
│   │   ├── types/                    # Shared TypeScript contracts
│   │   └── utils/                    # Path and MDX import helpers
│   ├── App.svelte
│   └── main.ts
├── src-tauri/                        # Backend (Rust/Tauri)
│   ├── Cargo.toml
│   ├── capabilities/default.json
│   └── src/
│       ├── commands/
│       │   ├── workspace.rs          # Workspace state, kind detection, workspace open
│       │   ├── documents.rs          # Read/write/parse/serialize and image resolution
│       │   ├── git.rs                # Git Sync command and structured outcomes
│       │   ├── secrets.rs            # GitHub PAT command boundary
│       │   └── mod.rs
│       ├── domain/
│       │   ├── document.rs           # DocumentRef, DocumentKind, ParsedDocument
│       │   ├── workspace.rs          # WorkspaceInfo, WorkspaceKind, CollectionInfo
│       │   └── frontmatter.rs        # YAML frontmatter parse/serialize
│       ├── services/
│       │   ├── fs.rs                 # Atomic read/write
│       │   ├── scanner.rs            # Bounded directory scanning
│       │   ├── astro.rs              # Astro detection and collections
│       │   ├── security.rs           # Canonical path and workspace-boundary validation
│       │   ├── git.rs                # Commit, fetch/merge, and push implementation
│       │   ├── secrets.rs            # OS credential-store adapter and service
│       │   └── mod.rs
│       ├── lib.rs
│       └── main.rs
├── docs/
│   ├── architecture/
│   │   ├── architecture.md
│   │   └── SPEC_TECNICO_FINAL.md     # Pointer to the canonical root spec
│   ├── product/rules.md
│   ├── prompts/MASTER_PROMPT.md      # Pointer to the canonical root prompt
│   └── rules/AGENTS.md               # Pointer to the canonical root contract
├── AGENTS.md                         # Canonical project contract
├── MASTER_PROMPT.md                  # Canonical implementation prompt
├── SPEC_TECNICO_FINAL.md             # Canonical technical specification
└── package.json
```

## Workspace state and path boundary

The backend owns the active workspace root in managed `WorkspaceState`.

1. The frontend uses the Tauri dialog plugin to choose a folder.
2. `open_workspace` receives that initial root, canonicalizes it, scans it, and stores it in backend state.
3. `detect_workspace_kind`, document commands, image resolution, and Git Sync use the backend-held root.
4. Document commands receive relative paths only; they do not receive a caller-selected root.
5. Git Sync receives a repository path, but the backend accepts it only when its canonical path equals the canonical workspace root exactly.

This prevents a later command from replacing the authorized root or escaping it.

## Tauri IPC contract

| Command | Input | Output |
|---------|-------|--------|
| `open_workspace` | `{ root }` | `WorkspaceInfo` |
| `detect_workspace_kind` | none | `{ kind, signals }` |
| `read_document` | `{ relativePath }` | `{ raw }` |
| `write_document` | `{ relativePath, raw }` | `{ ok: true }` |
| `parse_document` | `{ raw, kind }` | `ParsedDocument` |
| `serialize_document` | `{ frontmatter, body, kind }` | `{ raw }` |
| `resolve_image` | `{ fromFile, imagePath }` | `{ dataUrl, mime, size }` |
| `save_github_token` | `{ token }` | `void` |
| `clear_github_token` | none | `void` |
| `github_token_configured` | none | `boolean` |
| `sync_to_github` | `{ repoPath, commitMessage }` | `SyncOutcome` |

Folder selection is a frontend dialog operation, not a registered Tauri command. Workspace inventory is produced by `open_workspace`; there is no separate workspace-scan IPC command.

### Image resolution

`resolve_image` resolves a path relative to the referencing document and then, as a fallback, relative to the workspace root. Before reading bytes, the backend canonicalizes the candidate and verifies that it remains inside the workspace root. The frontend receives a `data:` URL, MIME type, and byte size rather than direct filesystem access.

### Secret management

The GitHub PAT crosses the frontend/backend boundary only when it is saved. `save_github_token` places it in the host operating-system credential store; `clear_github_token` removes it; `github_token_configured` returns only a boolean. The token is never returned by a command and is never persisted in frontend storage.

### Git Sync payload

The Git Sync request contains no token:

```ts
{
  repoPath: "<canonical path of the opened workspace repository>",
  commitMessage: "Update <document-name>"
}
```

The backend validates `repoPath` against the canonical workspace root before reading the PAT from the OS credential store. It then stages the repository, creates a commit, integrates the remote branch when possible, and pushes. The command returns a structured outcome: `synced`, `no_changes`, `auth_failed`, `scope_missing`, `conflict`, or `failed`.

Git Sync is manual and limited to the opened workspace repository. It does not synchronize other repositories and does not run automatically in the background.

## Security model

- All document paths are validated against the backend-held workspace root
- `..` traversal and absolute document paths are rejected
- Canonical path resolution with parent fallback for new files
- Atomic file writes (temporary file plus replacement)
- Image reads require canonical containment inside the workspace root
- Git repository paths must exactly match the canonical workspace root
- The PAT is stored in the OS credential store, not in `localStorage`
- File contents and secrets are not logged

## Frontend stores

| Store | Responsibility |
|-------|---------------|
| `workspaceStore` | Root, kind, collections, files, selected file |
| `documentStore` | Current document, frontmatter, body, dirty state, history |
| `uiStore` | Sidebar visibility, focus mode, preview mode |
| `preferencesStore` | Editor font, UI font |
| `githubTokenStore` | Whether a PAT is configured; never the PAT itself |
| `githubSyncStore` | Sync request state and structured result |
| `notifications` | Toast notifications (success, error, info) |

## Design tokens

```css
--ink #1C1917      /* warm near-black */
--paper #FAFAF8    /* warm off-white */
--chrome #F5F5F3   /* sidebar background */
--rule #E7E5E4     /* borders */
--accent #7C3AED   /* violet, focus states */
--muted #A8A29E    /* secondary text */
```
