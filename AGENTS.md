# AGENTS.md — Interstellar Writer Next (Tauri Prototype)

> **Fuente canónica:** este archivo en la raíz del repositorio. `docs/rules/AGENTS.md` es únicamente un puntero a esta fuente.

## Producto (contrato)

Interstellar Writer Next es un editor premium writer-first para carpetas locales con archivos `.md` y `.mdx`.

Principios:

- El editor es el protagonista.
- Todo lo que no sea escribir debe ser silencioso y secundario.
- Los paneles laterales existen para apoyar el flujo, no para competir con el texto.
- Focus mode debe limpiar el chrome de la app de forma notable.

## Alcance del prototipo (v1)

Incluye:

- abrir carpeta local (workspace)
- escaneo de `.md`/`.mdx`/`.mmd`/`.mermaid`
- abrir/editar/guardar
- YAML frontmatter: leer/editar/serializar sin romper
- preview markdown
- mermaid: render con `mermaid.js` o fallback elegante
- focus mode
- selector de tipografía (default: Newsreader para edición)
- Git Sync manual: commit y push disparados por el usuario hacia el repositorio del workspace abierto

No incluye:

- sincronización en la nube
- colaboración
- export/publish

## Git Sync en v1

- Es manual: solo se ejecuta cuando el usuario lo solicita.
- Opera únicamente sobre el repositorio del workspace abierto; la ruta del repositorio debe coincidir, después de canonicalizarla, con el `workspace root` autorizado por el backend.
- El PAT de GitHub se guarda en el almacén de credenciales del sistema operativo, nunca en `localStorage`.
- El token no forma parte del payload de sincronización: el backend lo obtiene internamente cuando ya validó el límite del workspace.

## Arquitectura (no negociable)

- Frontend (Svelte):
  - UI, estado visual, composición, interacción
  - no parsing ni reglas de filesystem
- Backend (Rust/Tauri):
  - filesystem, validación de paths, parsing y serialización
  - commands estrictos para el frontend

Astro:

- Astro support existe como adapter, no como “verdad del dominio”.
- El core funciona en una carpeta cualquiera con markdown.
- MDX se asume Astro-only (no React). El preview no ejecuta componentes; usa placeholders seguros.

## Seguridad

- Todas las rutas desde el frontend se validan contra el `workspace root`.
- Prohibido `..` traversal, rutas absolutas o escapes.
- Git Sync exige que la ruta canonicalizada del repositorio sea exactamente el `workspace root` abierto.
- El PAT de GitHub vive en el almacén de credenciales del sistema operativo, no en `localStorage` ni en el estado persistente del frontend.
- No loggear contenidos de archivos por defecto.

## Diseño UI

Benchmark:

- Astro Editor: tooling MDX + frontmatter forms + ergonomía writer-first
- iA Writer: tipografía-first + minimal chrome + foco
- Bear/Lettera: calidez + premium + quiet UI

Reglas:

- evitar bordes duros y cajas gratuitas
- espaciado generoso y jerarquía tipográfica clara
- una “writing surface” central tipo página

## Calidad

- tests de Rust para parse/serialize y validación de paths
- CI mínima: build + tests
- evitar dependencias pesadas si no aportan al prototipo
