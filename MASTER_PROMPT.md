# Master Prompt — Interstellar Writer Next

> **Canonical source:** this root file. `docs/prompts/MASTER_PROMPT.md` is only a pointer to this source.

You are building the separate **Interstellar Writer Next** repository.

Repository root: `<repo-root>` (the directory that contains this file).

Real validation workspace: `<validation-workspace>` (a local folder selected by the user; never hard-code a machine-specific path).

Goal: a cross-platform, premium, writer-first desktop editor built with **Tauri v2 + Svelte + Vite + Tailwind + TypeScript**, backed by **Rust commands** for filesystem and document parsing.

## Historical direction

The prototype was prepared in the previous editor project and then split into this repository so that its core could work against any local Markdown workspace. Preserve that separation: Astro support is an adapter, not a requirement of the core.

## Non-negotiables

1. Writer-first UX: the editor is the hero; UI chrome must stay quiet.
2. Real filesystem: must read and write real `.md` and `.mdx` files in a local folder.
3. YAML frontmatter: parse/edit/serialize safely.
4. Mermaid support: render if possible, otherwise graceful fallback (never break the page).
5. Cross-platform: Windows/macOS/Linux from day 1 (no platform-specific hacks).
6. Architecture boundary: frontend = UI; backend (Rust) = filesystem + parsing + validation.
7. Do not hard-code “Astro-only” assumptions into the core domain. Astro support must be an adapter.
8. Git Sync is user-triggered and limited to the repository of the opened workspace.

## Inspiration targets (blend, not clone)

- Astro Editor: schema-driven metadata forms, MDX-friendly workflows, image handling ergonomics.
- iA Writer: typography-first writing, focus mode, minimal chrome.
- Bear/Lettera: warm premium look, calm sidebars, subtle interaction design.

## Scope (Prototype v1 / Editor first-slice)

Must ship:

- Select workspace folder.
- Open and scan `.md`, `.mdx`, `.mmd`, and `.mermaid` files.
- Open a document.
- Edit body text.
- Parse frontmatter YAML and show it in a right-side form panel:
  - `title`, `description`, `date`, `draft`, `tags`.
- Save document back to disk safely.
- Markdown preview panel (toggle).
- Mermaid blocks:
  - If mermaid renderer is available, render it.
  - Else display a nice “Mermaid preview unavailable” block with original code.
- MDX preview rules (Astro-only, no React):
  - Do not execute components.
  - Render markdown normally and replace `<Component />` blocks with safe placeholders.
  - Preserve Astro-specific directives in placeholders (e.g. `client:*`) for visibility.
- Focus mode:
  - Hide sidebars and reduce chrome.
- Manual Git Sync:
  - Commit and push the opened workspace repository only when the user requests it.
  - Validate the repository path against the canonical workspace root in the backend.
  - Store the GitHub PAT in the operating-system credential store, not in `localStorage`.

Not required in v1:

- Cloud synchronization
- Collaboration
- Perfect MDX rendering
- Plugin system
- Automatic or background Git synchronization
- Git Sync against repositories other than the opened workspace repository

## Astro compatibility note

The previous editor already reads Astro collections. The prototype preserves that direction through an adapter boundary:

- Generic Markdown workspaces work first.
- Astro collection detection (`src/content/**`) is a dedicated adapter layer.

## Repository structure

```text
InterstellarWriterNext/
  src-tauri/
    Cargo.toml
    capabilities/
      default.json
    src/
      commands/
        workspace.rs
        documents.rs
        git.rs
        secrets.rs
      domain/
        document.rs
        workspace.rs
        frontmatter.rs
      services/
        fs.rs
        scanner.rs
        astro.rs
        security.rs
        git.rs
        secrets.rs
      lib.rs
      main.rs
  src/
    lib/
      api/
      components/
        editor/
        layout/
        preview/
        shared/
      features/
        preview/
      stores/
      styles/
      types/
    App.svelte
    main.ts
  docs/
    architecture/
    product/
    prompts/
    rules/
  AGENTS.md
  MASTER_PROMPT.md
  SPEC_TECNICO_FINAL.md
```

The root `AGENTS.md`, `MASTER_PROMPT.md`, and `SPEC_TECNICO_FINAL.md` files are canonical. Their counterparts under `docs/` are pointers only.

## Rust backend contract (commands)

All filesystem commands run in Rust. The backend keeps the canonical workspace root in managed state after `open_workspace`; document commands receive paths relative to that root and never receive a caller-selected root.

- `detect_workspace_kind() -> { kind, signals }`
- `open_workspace({ root }) -> WorkspaceInfo`
- `read_document({ relativePath }) -> { raw }`
- `write_document({ relativePath, raw }) -> { ok }`
- `parse_document({ raw, kind }) -> ParsedDocument`
- `serialize_document({ frontmatter, body, kind }) -> { raw }`
- `resolve_image({ fromFile, imagePath }) -> { dataUrl, mime, size }`
- `save_github_token({ token }) -> void`
- `clear_github_token() -> void`
- `github_token_configured() -> boolean`
- `sync_to_github({ repoPath, commitMessage }) -> SyncOutcome`

The Git Sync payload contains no token. The backend reads the PAT from the operating-system credential store only after the canonical repository path has been validated against the opened workspace root.

## Security rules

- Canonicalize the workspace root when it is opened and retain it in backend state.
- Validate every document path against that root.
- Disallow `..` traversal and absolute paths from the frontend.
- For Git Sync, require the canonical repository path to equal the canonical workspace root exactly.
- Keep the GitHub PAT in the operating-system credential store; never persist it in `localStorage` or include it in the sync payload.
- Never log secrets or file contents in production mode.

## Frontmatter parsing strategy

Port proven Rust logic from the existing editor:

- `parse_content` and `serialize_content` equivalents (YAML delimited by `---`).
- Remove nulls on serialization.
- Preserve body formatting by not trimming aggressively.

## UI requirements (Tailwind)

- Typography-first:
  - Comfortable line height
  - Restrained UI density
  - Strong hierarchy for headings
- Quiet chrome:
  - Very few borders
  - Subtle separators
  - Calm colors
- Sidebars:
  - Left: navigation
  - Right: metadata forms
  - Both must be collapsible
- Provide a single “writing surface” component that looks like a page.
- Provide a font selector:
  - Default editor font: Newsreader
  - User can switch fonts (persist per workspace or globally)
  - Initial set: Newsreader + Inter + JetBrains Mono

## Testing expectations

- Rust unit tests for:
  - Frontmatter parse/serialize
  - Path security validation
  - Workspace-root enforcement for Git Sync
  - Secret-service behavior through an in-memory test store
- Frontend type checking with `svelte-check`.
- Manual runtime checks remain necessary for the real OS credential store and a real GitHub push.

## Deliverable

Produce:

1. A runnable Tauri app that meets the v1 scope above.
2. Clean architecture boundaries between frontend and backend.
3. Canonical root documentation plus the documentation tree under `docs/`.
