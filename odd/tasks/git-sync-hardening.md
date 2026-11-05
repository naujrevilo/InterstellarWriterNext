# Git Sync Hardening and Delivery Closure

Stabilize Rust verification, enforce the workspace-root path boundary for Git Sync, and close the approved scope, documentation, and CI gaps.
The work is tracked in the order authorized by the human: toolchain, path validation, then scope/documentation/CI closure.

## Specs

- S1: "Ejecuta tus acciones recomendadas en ese orden"
- S2: "Por otra parte, no combines idiomas. Español de Colombia solamente. Escribiste en portugues una palabra, creo que una, así que ten presente eso"
- S3: "1 y 2. https://github.com/naujrevilo/InterstellarWriterNext"
- S4: "3. Tengo como probar en Linux, Mac no"
- S5: "Puedo ejecutar localmente para ver cómo quedó?"

## Tasks

- T1 — S1 — route: inline/toolchain — commit: N/A (no usable source Git history; `cargo test`: 40/40 correctas)
- T2 — S1 — route: inline/test-first — commit: N/A (no usable source Git history; `cargo test`: 43/43 correctas)
- T3 — S1 — route: inline/frontend — commit: N/A (no usable source Git history; `svelte-check`: 0 errores)
- T4 — S1 — route: inline/frontend — commit: N/A (no usable source Git history; `svelte-check`: 0 errores)
- T5 — S1 — route: inline/frontend — commit: N/A (no usable source Git history; build: 0 errores; fuentes y licencias empaquetadas)
- T6 — S1 — route: inline/security-review — commit: N/A (no usable source Git history; `cargo test`: 55/55; verificación independiente sin defectos confirmados)
- T7 — S1 — route: inline/frontend — commit: N/A (no usable source Git history; `svelte-check`: 0 errores, 0 avisos)
- T8 — S1 — route: inline/documentation — commit: N/A (no usable source Git history; `svelte-check`: 0 errores, 0 avisos; comprobación estructural correcta)
- T9 — S1 — route: inline/ci — commit: N/A (no usable source Git history; YAML válido, matriz y comandos verificados; primera ejecución alojada pendiente)
- T10 — S1-S2 — route: inline/verification — commit: N/A (no usable source Git history; `cargo test --locked`: 57 passed, 0 failed, 1 ignored; probe del sistema operativo: 1 passed; `svelte-check`: 0/0; build y CI: correctos)
- T11 — S3-S4 — route: inline/repository-delivery — commit: `ec4ff85` (rama `feature/stabilize-v1`; cinco unidades desde `0db3726`)
- T12 — S5 — route: inline/local-runtime — commit: N/A
- T13 — S3 — route: inline/live-sync — commit: pending
- T14 — S3-S4 — route: inline/hosted-ci — commit: pending

## Log

- L1 — 2026-11-05 — "Ejecuta tus acciones recomendadas en ese orden\nPor otra parte, no combines idiomas. Español de Colombia solamente. Escribiste en portugues una palabra, creo que una, así que ten presente eso"
- L2 — Scope decision: retain Git Sync in v1. The authorized sequence explicitly requires validating `repo_path` before closing scope, which removes the keep/remove ambiguity described in the prior status report.
- L3 — Toolchain evidence: the failed native link was caused by `CC=zig cc` compiling `libgit2-sys` objects for an MSVC link; targeted artifact cleanup alone cannot correct the mixed-toolchain cause.
- L4 — Repository constraint: the workspace `.git` directory contains only the Engram project identity, with no commits, refs, or remote; it is not a usable source repository, so work-unit commits and branch creation remain unavailable without separate repository authority.
- L5 — T1 evidence: rebuilt `libgit2-sys`, `libssh2-sys`, and `git2` with the Visual Studio 2022 environment and `cl.exe`; `cargo test` completed with 40 passed, 0 failed. The prior `LNK1143` failure was caused by the inherited `CC=zig cc` setting.
- L6 — T2 evidence: observed RED for the missing path-validation boundary, implemented canonical workspace-root validation in the backend, and verified that an outside repository keeps its HEAD and untracked state. The existing `NoChanges` outcome remains covered. `cargo test`: 43 passed, 0 failed.
- L7 — Plan expansion: the authorized request covers the complete ordered recommendation list from the status report, not only the final three-part handoff. Added explicit tasks for the unsaved-changes guard, shortcut collision, font delivery, Git credential storage evaluation, accessibility warnings, documentation, CI, and local sync verification. Documentation inspection started before this expansion but did not modify source or documentation files.
- L8 — T3 evidence: added a native Tauri discard confirmation before changing documents, opening another workspace, or resetting the workspace. Declining or failing the prompt preserves the current document; selecting the already-open document does not prompt or reload. Frontend has no test runner, so RED was unavailable by design; `svelte-check` completed with 0 errors and the production build passed.
- L9 — T4 evidence: Ctrl+B and Ctrl+I now reuse the existing toolbar Markdown transformations inside the writing surface, with the same text and caret placement as the buttons. The global panel shortcuts remain active outside editable elements. `svelte-check` completed with 0 errors and the production build passed.
- L10 — T5 evidence: bundled official Newsreader, Inter, and JetBrains Mono files locally, removed all Google Fonts runtime links, and packaged the three OFL license texts under `dist/licenses/`. Source and packaged font hashes match; no Google Fonts URL remains in source or `dist/`; the production build passed.
- L11 — T6 evidence: moved the GitHub PAT into the operating-system credential store through `keyring` 4.2, removed it from frontend preference state and the sync payload, and added one-time legacy migration plus immediate scrubbing. RED: 47 passed / 1 failed for empty-token rejection. GREEN: 55 passed / 0 failed. An independent verifier found no confirmed defects; the two low-severity advisories were fixed by scrubbing any legacy field and preserving an already-stored OS credential. Real OS credential-store round-trip remains a manual runtime check.
- L12 — T7 evidence: replaced the clickable non-interactive modal backdrop and inner stop-propagation handler with a dedicated accessible dismiss button behind the dialog. Click-outside close remains available. `svelte-check`: 0 errors, 0 warnings.
- L13 — T8 evidence: retained manual Git Sync in v1 and aligned the canonical root documents, product rules, architecture, IPC type contract, bootstrap reference, and checklist with the implementation. The three duplicated files under `docs/` now point to canonical root sources. Removed stale Windows drive paths, obsolete command signatures, Git exclusions, and npm prescriptions. `svelte-check`: 0 errors, 0 warnings; structural grep passed. Also removed the last extra `root` argument sent to `resolve_image`.
- L14 — T9 evidence: added `.github/workflows/ci.yml` for Windows/macOS/Ubuntu with pnpm 11.18.0, Node 22, stable Rust, frontend check/build, Rust tests, Cargo/pnpm caching, and Linux Tauri dependencies. Expanded `.gitignore` and aligned `CI_PLAN.md`, `CHECKLIST_V1.md`, and `SPEC_TECNICO_FINAL.md`. YAML parsed successfully and 31 structural assertions passed. The hosted first run remains pending because the workspace has no usable source Git history.
- L15 — T10 evidence: added and passed a deterministic local bare-remote push test and an explicitly-run OS credential-store probe with a unique temporary key and cleanup guard. Final checks: `cargo test --locked` 57 passed / 0 failed / 1 ignored, explicit OS probe 1 passed, `svelte-check` 0 errors / 0 warnings, Vite production build successful, CI YAML valid and structurally verified, and documentation checks passed. The final independent verifier accepted the feature; its three minor findings were fixed: removed the last stale Windows path from the UI, added the remaining standard Linux Tauri packages, and redacted credentials from remote URLs in diagnostics.
- L16 — External proof still pending: hosted CI on the three runners, macOS/Linux UI smoke tests, macOS/Linux credential-store round-trips, and a real authenticated GitHub push. The workspace `.git` contains only the Engram identity and has no commits, refs, or remote, so no work-unit commits or hosted CI run could be created here.
- L17 — 2026-11-05 — "1 y 2. https://github.com/naujrevilo/InterstellarWriterNext\n3. Tengo como probar en Linux, Mac no\nPuedo ejecutar localmente para ver cómo quedó?"
- L18 — Remote evidence: `https://github.com/naujrevilo/InterstellarWriterNext.git` resolves to `main` at `0db3726`, containing only a one-line README. The GitHub CLI is authenticated as `naujrevilo` with `repo` and `workflow` scopes; no credential value was logged.
- L19 — Platform evidence: Linux can be used for manual verification. macOS remains unavailable and must stay explicitly pending.
- L20 — T11 evidence: added `origin` for `https://github.com/naujrevilo/InterstellarWriterNext.git`, created `feature/stabilize-v1` from the remote `main` commit `0db3726`, and published five reviewable commits: frontend, Rust/Tauri backend, v1 documentation, CI/hygiene, and ODD tracking. The branch is tracked at `origin/feature/stabilize-v1`; `main` remains unchanged.
