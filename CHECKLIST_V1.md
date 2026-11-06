# Checklist — Prototipo v1 (Editor first-slice)

> **Corte de evidencia:** tareas T1–T10 del ODD `odd/tasks/git-sync-hardening.md`.
>
> - `[x]` significa “implementado y cubierto por evidencia automática o inspección estructural”.
> - `[ ]` significa “pendiente”.
> - La compilación y las pruebas no sustituyen un smoke test manual de UI.

## UX / Producto

- [x] Abrir un workspace local y listar archivos `.md`, `.mdx`, `.mmd` y `.mermaid`.
  - Evidencia: pruebas Rust de apertura/clasificación del workspace y escaneo; frontend compilado con `svelte-check`.
- [x] Sidebar izquierda con archivos y colecciones.
- [x] Editor central con writing surface y protección contra cambios sin guardar.
- [x] Panel derecho con metadatos editables por colección.
- [x] Focus mode, preview Markdown, Mermaid con fallback y placeholders MDX seguros implementados.
  - Evidencia: `svelte-check` sin errores y build de producción exitoso; no equivale a una prueba manual de interacción.
- [ ] Smoke test manual completo de la UI en Windows.
  - Pendiente: no se registra un recorrido manual completo.

## Integridad de datos

- [x] Parse de frontmatter YAML sin romper YAML válido.
  - Evidencia: pruebas Rust de parsing y serialización.
- [x] Serialización que preserva el body y limpia `null`.
  - Evidencia: pruebas Rust de frontmatter.
- [x] Guardado atómico mediante archivo temporal y reemplazo.
  - Evidencia: pruebas Rust de filesystem y round-trip.
- [x] Dirty tracking, undo/redo y aviso al descartar cambios implementados.
  - Evidencia: implementación TypeScript/Svelte y verificación de tipos; falta validación manual del flujo.

## Seguridad

- [x] Validación de rutas relativas dentro del `workspace root`.
  - Evidencia: pruebas Rust rechazan rutas absolutas, traversal y escapes.
- [x] Git Sync limitado a la ruta canónica exacta del repositorio abierto.
  - Evidencia: pruebas Rust prueban rechazo de repositorios externos y ausencia de mutación.
- [x] Resolución de imágenes locales contenida en el workspace.
  - Evidencia: pruebas Rust de resolución y rechazo fuera de raíz.
- [x] PAT de GitHub en el servicio de credenciales del sistema operativo y nunca en `localStorage`.
  - Evidencia: pruebas del servicio con store en memoria e inspección estructural del frontend.
- [x] Round-trip real en el almacén de credenciales del sistema operativo en Windows (host de esta revisión).
  - Evidencia: probe ignorado `services::secrets::os_store_probe::os_credential_store_round_trip`, ejecutado de forma
    explícita en este host: escribe una cuenta de prueba única bajo el nombre de servicio de la aplicación, la lee,
    la elimina y comprueba que ya no existe. El Administrador de credenciales de Windows mostró la entrada durante la
    escritura y no dejó ninguna entrada de `interstellar-writer-next` al terminar. La clave real `github-token` nunca
    se usó. La prueba queda `#[ignore]`: no corre en la suite normal ni en CI.
- [ ] Round-trip real en el almacén de credenciales de macOS y Linux.
  - Pendiente: falta ejecutar el mismo probe en esos sistemas operativos.

## Git Sync

- [x] Flujo manual de commit y push disparado por el usuario hacia el workspace abierto.
  - Evidencia: comando Rust, estados de resultado y guards de raíz implementados.
- [x] Payload de Git Sync sin token; el backend obtiene el PAT internamente después de validar la ruta.
  - Evidencia: inspección estructural del contrato IPC y de los servicios.
- [x] Resultados estructurados para `synced`, `no_changes`, `auth_failed`, `scope_missing`, `conflict` y `failed`.
  - Evidencia: pruebas Rust de camino infiel, repositorio limpio y fallos de credencial.
- [x] Push verificado contra un remoto bare local, sin red y sin GitHub.
  - Evidencia: prueba `services::git::tests::commit_and_push_reaches_a_local_bare_remote`: crea un remoto bare local y
    un repositorio normal con identidad configurada en el propio repositorio (sin depender de la configuración global
    de Git), hace un commit inicial, modifica el worktree, invoca el servicio real `commit_and_push` y comprueba que
    la rama remota recibió el commit, con su mensaje, su padre y el contenido modificado. Esto **no** es un push a
    GitHub.
- [x] Push real a GitHub.
  - Evidencia: el servicio real `commit_and_push` publicó `5149f20` en `origin/feature/stabilize-v1`; GitHub confirma el commit
    y sus archivos (`README.md` y `odd/tasks/git-sync-hardening.md`).

## Cross-platform

- [ ] Windows: smoke test manual de UI.
- [ ] macOS: smoke test manual de UI.
- [ ] Linux: smoke test manual de UI.

No se marca ningún smoke test de UI como completo porque no hay evidencia de un recorrido manual completo en esta revisión.

La cobertura automática de las tres plataformas se ejecutó en GitHub Actions (matriz `windows-latest`,
`macos-latest` y `ubuntu-latest`) y terminó correctamente. Esto no sustituye los recorridos manuales de interfaz.

## Calidad

- [x] Tests Rust para parse/serialize, filesystem, seguridad, workspace, Git Sync y secrets.
  - Evidencia registrada: `cargo test --locked`, 57 passed / 0 failed / 1 ignored (el probe del almacén real del
    sistema operativo queda ignorado por diseño y solo se ejecuta de forma explícita).
- [x] Verificación TypeScript/Svelte.
  - Evidencia registrada: `svelte-check`, 0 errores y 0 avisos.
- [x] Build de producción frontend.
  - Evidencia registrada: build exitoso y fuentes/licencias de tipografías empaquetadas.
- [x] Workflow de CI de build y tests configurado en `.github/workflows/ci.yml`.
  - Evidencia: matriz Windows/macOS/Ubuntu, disparadores de `push` y `pull_request`, y validación estructural del YAML
    con confirmación de los comandos `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm build` y
    `cargo test --manifest-path src-tauri/Cargo.toml --locked`. Equivalentes locales ejecutados: `svelte-check`,
    `vite build` y `cargo test --locked`.
  - Evidencia registrada (T10): el YAML se analiza sin error y las 51 aserciones estructurales pasan; los equivalentes
    locales se ejecutaron de forma directa (`svelte-check` con 0 errores y 0 avisos, `vite build` exitoso y
    `cargo test --locked` con 57 passed / 0 failed / 1 ignored).
- [x] Primera ejecución alojada del workflow de CI.
  - Evidencia: ejecución `42105342005` completada correctamente en Windows, macOS y Ubuntu:
    https://github.com/naujrevilo/InterstellarWriterNext/actions/runs/42105342005
