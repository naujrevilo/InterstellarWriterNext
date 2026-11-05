# Product Rules — Interstellar Writer Next

## Core principle

The editor is the protagonist. Everything that is not writing must be silent and secondary.

## Writer-first UX

- The writing surface is a centered page with generous margins
- Typography takes priority: Newsreader for editing, Inter for UI, JetBrains Mono for code
- Sidebars exist to support the flow, not compete with the text
- Focus mode removes all chrome and leaves the editor as the absolute center

## Quiet chrome

- Very few borders, subtle separators
- Calm colors: warm near-black (#1C1917), off-white paper (#FAFAF8)
- No gratuitous boxes or hard edges
- Generous spacing and clear typographic hierarchy

## Focus mode

- Hides both sidebars completely
- Reduces toolbar to minimal controls
- Background shifts to warm paper tone
- Escape key or click to exit
- Keyboard shortcut: Ctrl+Shift+F

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+S | Save document |
| Ctrl+Shift+F | Toggle focus mode |
| Ctrl+Shift+P | Toggle preview |
| Escape | Exit focus mode |

## File handling

- Open workspace via folder picker dialog
- Scan `.md`, `.mdx`, `.mmd`, `.mermaid` files
- Exclude: `node_modules`, `.git`, `dist`, `build`, `.astro`, `.next`, `target`
- Automatic title extraction from frontmatter

## Frontmatter

- Parse YAML delimited by `---`
- Edit via metadata panel (right sidebar)
- Serialize without breaking formatting
- Clean null values on save
- Collection-aware fields (blog, app-landings, projects, docs)

## Preview

- Markdown rendered with `marked`
- Mermaid diagrams rendered client-side with graceful fallback
- MDX components shown as safe placeholders (never executed)
- Astro directives preserved in placeholders

## Git Sync (v1)

- Manual, user-triggered commit and push to the repository of the opened workspace
- The backend accepts only the canonical workspace root as the repository path
- The GitHub PAT is stored in the operating-system credential store, never in `localStorage`
- The sync payload contains no token; the backend reads the credential internally after path validation
- No automatic, background, or multi-repository synchronization

## Not included in v1

- Cloud synchronization
- Collaboration
- Export/publish
- Plugin system
- Full MDX rendering
