# Interstellar Writer Next

Editor de escritorio writer-first para carpetas locales de Markdown y MDX, construido con Tauri v2, Rust y Svelte 5.

## Desarrollo local

Requisitos:

- Node.js 22
- pnpm 11.18.0
- Rust estable
- En Linux, las dependencias de sistema de Tauri v2

Instala las dependencias y ejecuta las comprobaciones:

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

### Windows

Ejecuta el lanzador incluido para activar `cl.exe` desde Visual Studio antes de iniciar Tauri:

```bat
scripts\dev-windows.cmd
```

### Linux y macOS

```bash
pnpm tauri dev
```

La interfaz de desarrollo queda disponible en `http://localhost:1420`.

## Crear documentos

Abre un workspace y pulsa **New document** en el explorador. En proyectos Astro, también puedes pulsar **+** junto a una colección para crear un archivo en ella. Elige título, nombre y formato `.md` o `.mdx`; el archivo se crea sin sobrescribir otro y se abre para editarlo.

El documento nuevo contiene inicialmente solo el título en su frontmatter y no se marca automáticamente como borrador. Completa en **Metadata** los campos que requiera tu colección; **Other fields** permite agregar y editar valores adicionales en JSON. En carpetas Markdown genéricas puedes indicar una carpeta relativa existente o crear el archivo en la raíz.

## Git Sync

Git Sync es manual y solo opera sobre el repositorio del espacio de trabajo abierto. La ruta se valida en Rust contra la raíz canónica autorizada. El token de GitHub se guarda en el almacén de credenciales del sistema operativo, nunca en `localStorage`.

## Documentación

- `AGENTS.md`: contrato del producto y reglas de implementación.
- `SPEC_TECNICO_FINAL.md`: especificación técnica canónica.
- `CHECKLIST_V1.md`: estado verificado y pendientes manuales.
- `CI_PLAN.md`: estrategia de CI para Windows, macOS y Ubuntu.
