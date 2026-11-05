# Spec técnico final — Interstellar Writer Next

> **Fuente canónica:** este archivo en la raíz del repositorio. `docs/architecture/SPEC_TECNICO_FINAL.md` es únicamente un puntero a esta fuente.

## 1. Objetivo

Construir un prototipo desktop **cross-platform** en la raíz de este repositorio (`<repo-root>`) con:

- `Tauri v2`
- `Svelte`
- `Vite`
- `Tailwind CSS`
- `TypeScript`
- backend Rust para filesystem, parsing y seguridad

El prototipo debe validar una UX **writer-first premium** con soporte real para:

- `.md`
- `.mdx`
- `frontmatter YAML`
- `mermaid`
- detección de colecciones Astro

Workspace real de validación:

- `<validation-workspace>`: una carpeta local elegida por el usuario, sin rutas dependientes de una máquina.

## 2. Registro histórico del cierre

Esta especificación nació al cerrar la fase de preparación del editor anterior y separar el prototipo en `InterstellarWriterNext`. Ese hito ya quedó superado.

Para continuar, las decisiones vigentes son las secciones de arquitectura, contratos Tauri, seguridad, alcance v1 y pruebas de este documento. Las secciones de producto y UI conservan la narrativa original siempre que no contradigan la implementación actual.

## 3. Arquitectura

### 3.1 Principio rector

- **Frontend Svelte**:
  - UI
  - estado visual
  - interacción
  - composición de pantallas
- **Backend Rust/Tauri**:
  - filesystem
  - seguridad de paths
  - parsing de documentos
  - serialización
  - detección de workspace
  - integración Git y manejo seguro de credenciales

### 3.2 Separación por capas

```text
UI Layer
  -> Svelte components, stores, view-models

Application Layer
  -> action handlers, invoke wrappers, mapping backend/frontend

Domain Layer
  -> Document, Workspace, Collection, MetadataField, Adapter contracts

Infrastructure Layer
  -> Tauri commands, filesystem, parser, serializer, Mermaid helpers, Git, secrets
```

### 3.3 Regla no negociable

El frontend **nunca** toca disco directamente.

El backend conserva el `workspace root` canónico en estado administrado. `open_workspace` recibe la raíz inicial; los comandos posteriores de documentos reciben rutas relativas y no aceptan otra raíz.

## 4. Estructura de carpetas

```text
InterstellarWriterNext/
├─ .github/
│  └─ workflows/
│     └─ ci.yml                      # CI: matriz Windows/macOS/Ubuntu
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ capabilities/
│  │  └─ default.json
│  └─ src/
│     ├─ lib.rs
│     ├─ main.rs
│     ├─ commands/
│     │  ├─ mod.rs
│     │  ├─ workspace.rs
│     │  ├─ documents.rs
│     │  ├─ git.rs
│     │  └─ secrets.rs
│     ├─ domain/
│     │  ├─ mod.rs
│     │  ├─ workspace.rs
│     │  ├─ document.rs
│     │  └─ frontmatter.rs
│     └─ services/
│        ├─ mod.rs
│        ├─ fs.rs
│        ├─ scanner.rs
│        ├─ astro.rs
│        ├─ security.rs
│        ├─ git.rs
│        └─ secrets.rs
├─ src/
│  ├─ main.ts
│  ├─ App.svelte
│  └─ lib/
│     ├─ api/
│     │  └─ tauri.ts
│     ├─ components/
│     │  ├─ editor/
│     │  ├─ layout/
│     │  ├─ preview/
│     │  └─ shared/
│     ├─ features/
│     │  └─ preview/
│     ├─ stores/
│     ├─ styles/
│     └─ types/
├─ docs/
│  ├─ architecture/
│  │  ├─ architecture.md
│  │  └─ SPEC_TECNICO_FINAL.md
│  ├─ product/
│  │  └─ rules.md
│  ├─ prompts/
│  │  └─ MASTER_PROMPT.md
│  └─ rules/
│     └─ AGENTS.md
├─ AGENTS.md
├─ MASTER_PROMPT.md
└─ SPEC_TECNICO_FINAL.md
```

Los tres documentos de la raíz son las fuentes canónicas. Las copias bajo `docs/` son punteros breves.

## 5. Modelo de dominio

### 5.1 Rust

```rust
pub enum DocumentKind {
    Markdown,
    Mdx,
    MermaidFile,
}

pub enum WorkspaceKind {
    GenericMarkdown,
    Astro,
}

pub struct DocumentRef {
    pub relative_path: String,
    pub kind: DocumentKind,
    pub collection: Option<String>,
    pub title_hint: Option<String>,
    pub is_draft: bool,
}

pub struct ParsedDocument {
    pub frontmatter: serde_yaml::Mapping,
    pub body: String,
    pub raw: String,
}

pub struct WorkspaceInfo {
    pub root: String,
    pub kind: WorkspaceKind,
    pub collections: Vec<CollectionInfo>,
    pub files: Vec<DocumentRef>,
}

pub struct CollectionInfo {
    pub name: String,
    pub relative_path: String,
    pub document_count: usize,
}
```

### 5.2 TypeScript

```ts
export type DocumentKind = 'markdown' | 'mdx' | 'mermaid-file';
export type WorkspaceKind = 'generic-markdown' | 'astro';

export interface DocumentRef {
  relativePath: string;
  kind: DocumentKind;
  collection?: string | null;
  titleHint?: string | null;
  isDraft?: boolean;
}

export interface CollectionInfo {
  name: string;
  relativePath: string;
  documentCount: number;
}

export interface WorkspaceInfo {
  root: string;
  kind: WorkspaceKind;
  collections: CollectionInfo[];
  files: DocumentRef[];
}

export interface ParsedDocument {
  frontmatter: Record<string, unknown>;
  body: string;
  raw: string;
}
```

## 6. Contratos Tauri

### 6.1 Estado del workspace

`WorkspaceState` se administra en el backend y conserva la raíz canónica del workspace abierto.

- `open_workspace` es el único comando que recibe la raíz inicial.
- `detect_workspace_kind` usa la raíz del estado backend.
- Los comandos de documentos usan rutas relativas; la resolución de imágenes acepta la ruta del documento o una ruta absoluta contenida en la raíz.
- La selección de carpeta no es un comando IPC: es una operación del plugin de diálogo del frontend.

### 6.2 `open_workspace`

Abre una carpeta, la canonicaliza, la registra en el estado backend y devuelve el inventario inicial.

```ts
type OpenWorkspaceInput = {
  root: string;
};

type OpenWorkspaceResult = WorkspaceInfo;
```

### 6.3 `detect_workspace_kind`

Determina si el workspace abierto es Astro o Markdown genérico.

```ts
type DetectWorkspaceKindResult = {
  kind: 'astro' | 'generic-markdown';
  signals: string[];
};
```

Reglas:

- Astro si encuentra alguna combinación razonable de:
  - `src/content/`
  - `astro.config.*`
  - `src/content.config.*`
  - `package.json` con dependencia `astro`

### 6.4 Lectura y escritura de documentos

```ts
type ReadDocumentInput = {
  relativePath: string;
};

type ReadDocumentResult = {
  raw: string;
};

type WriteDocumentInput = {
  relativePath: string;
  raw: string;
};

type WriteDocumentResult = {
  ok: true;
};
```

Reglas:

- `relativePath` se valida dentro del `workspace root` backend.
- Se rechazan rutas absolutas y traversal con `..`.
- La escritura usa un archivo temporal y reemplazo atómico.

### 6.5 Parsing y serialización

```ts
type ParseDocumentInput = {
  raw: string;
  kind: DocumentKind;
};

type ParseDocumentResult = ParsedDocument;

type SerializeDocumentInput = {
  frontmatter: Record<string, unknown>;
  body: string;
  kind: DocumentKind;
};

type SerializeDocumentResult = {
  raw: string;
};
```

Notas:

- para `.md` y `.mdx`: usar delimitadores `---`
- para `.mmd`/`.mermaid`: `frontmatter = {}`, `body = raw`
- limpiar `null` al serializar
- mantener el body intacto salvo normalización mínima
- para `.mmd`/`.mermaid` devolver `body` puro

### 6.6 `resolve_image`

Resuelve una imagen local para el preview sin entregarla como ruta de filesystem al navegador.

```ts
type ResolveImageInput = {
  fromFile: string;
  imagePath: string;
};

type ResolveImageResult = {
  dataUrl: string;
  mime: string;
  size: number;
};
```

Reglas:

- `fromFile` es la ruta relativa del documento que referencia la imagen.
- Se prueba primero la ruta relativa al directorio del documento y luego la relativa a la raíz.
- Una ruta absoluta solo se acepta si su destino canonicalizado permanece dentro de la raíz.
- El comando devuelve bytes codificados como `data:` URL, MIME y tamaño.

### 6.7 Comandos de credenciales

```ts
type SaveGithubTokenInput = {
  token: string;
};

type ClearGithubTokenResult = void;
type GithubTokenConfiguredResult = boolean;
```

- `save_github_token({ token })` guarda el PAT en el almacén de credenciales del sistema operativo.
- `clear_github_token()` elimina el PAT almacenado.
- `github_token_configured()` responde únicamente si existe un token configurado.
- Ningún comando devuelve el valor del PAT.

### 6.8 `sync_to_github`

```ts
type SyncToGithubInput = {
  repoPath: string;
  commitMessage: string;
};

type SyncOutcome =
  | { kind: 'synced'; message: string }
  | { kind: 'no_changes' }
  | { kind: 'auth_failed'; reason: string }
  | { kind: 'scope_missing'; reason: string }
  | { kind: 'conflict'; reason: string }
  | { kind: 'failed'; reason: string };
```

El payload de Git Sync **no contiene token**. `repoPath` debe canonicalizar exactamente al `workspace root` que el backend ya tiene registrado; cualquier otra ruta se rechaza antes de consultar la credencial. El PAT se obtiene internamente del almacén del sistema operativo para la operación de Git.

Git Sync es manual: el usuario dispara commit y push; no hay sincronización automática ni en segundo plano.

## 7. Seguridad

### 7.1 Estado canónico y validación de paths

- La raíz se canonicaliza al abrir el workspace y queda en backend state.
- `relativePath` no puede:
  - ser absoluto
  - contener traversal con `..`
  - salir del `workspace_root` tras normalización
- Los archivos nuevos se validan contra el ancestro existente más cercano.

### 7.2 Resolución de imágenes

- Toda imagen local se canonicaliza antes de leer bytes.
- El destino debe estar contenido en el `workspace root`.
- El frontend recibe un `data:` URL, no acceso directo de filesystem.

### 7.3 Git Sync

- Solo se sincroniza el repositorio cuya ruta canónica sea exactamente la raíz del workspace abierto.
- La comprobación ocurre antes de leer el PAT.
- El PAT vive en Windows Credential Manager, macOS Keychain o Secret Service, según el sistema operativo.
- El PAT no se guarda en `localStorage` y no forma parte del payload de sincronización.

### 7.4 Logging

- nunca loggear contenido completo
- no loggear tokens
- loggear:
  - operación
  - ruta relativa
  - duración
  - error resumido

## 8. Adapter Astro

### 8.1 Propósito

El adapter Astro especializa el comportamiento del core cuando detecta estructura Astro.

### 8.2 Responsabilidades

- detectar `src/content/**`
- derivar colecciones por carpeta
- enriquecer `DocumentRef.collection`
- elegir widgets de metadata por colección
- reconocer patrones típicos de MDX Astro

### 8.3 Reglas por colección

#### `blog`

Campos prioritarios:

- `title`
- `date`
- `description`
- `image`
- `tags`
- `categories`

Widgets:

- `title`: text
- `description`: textarea
- `date`: datetime
- `image`: path input
- `tags`: string array
- `categories`: string array

#### `app-landings`

Campos prioritarios:

- `title`
- `description`
- `longDescription`
- `technologies`
- `status`
- `primaryCtaLabel`
- `primaryCtaUrl`
- `secondaryCtaLabel`
- `secondaryCtaUrl`
- `repoUrl`
- `image`
- `pathSlug`

#### `projects`

Campos prioritarios:

- `title`
- `description`
- `longDescription`
- `publishDate`
- `technologies`
- `category`
- `status`
- `repoUrl`
- `demoUrl`
- `image`
- `featured`

### 8.4 MDX Astro-only

Regla:

- no asumir React
- no ejecutar componentes en preview v1
- renderizar placeholders seguros como:
  - `Component: Notice`
  - `Directive: client:load`
  - `Props summary`

### 8.5 `content.config.ts`

En v1:

- no se parsea Zod completo
- opcionalmente se detecta existencia del archivo
- el schema fuerte queda para v2

## 9. Preview

### 9.1 Markdown

- render completo

### 9.2 MDX

- render markdown normal
- reemplazar componentes por placeholders
- no romper el preview si aparece JSX/astro syntax

### 9.3 Mermaid

- bloques ```mermaid y archivos `.mmd` / `.mermaid`
- render client-side con `mermaid.js`
- si falla:
  - mostrar bloque elegante
  - explicar que el render no estuvo disponible
  - conservar el código

## 10. UI y layout

### 10.1 Layout

- izquierda:
  - colecciones
  - archivos
- centro:
  - header documental
  - editor
- derecha:
  - metadatos
- preview:
  - toggleable

### 10.2 Focus mode

Debe:

- ocultar sidebars
- reducir top chrome
- esconder ruido visual
- dejar el editor como centro absoluto

### 10.3 Tipografía

Selector de fuentes:

- `Newsreader` -> default del editor
- `Inter` -> UI general
- `JetBrains Mono` -> código y bloques técnicos

Persistencia:

- preferencia global v1
- per-workspace puede quedar para v2

## 11. Stores frontend

### 11.1 `workspaceStore`

Responsable de:

- root actual
- kind (`astro` / `generic-markdown`)
- colecciones
- archivos
- selección actual

### 11.2 `documentStore`

- documento actual
- frontmatter
- body
- dirty
- kind
- preview state

### 11.3 `uiStore`

- left sidebar open/close
- right sidebar open/close
- focus mode
- preview mode

### 11.4 `preferencesStore`

- fuente editor
- fuente UI
- theme

### 11.5 Stores de Git

- estado de si el PAT está configurado, sin conservar el secreto
- estado de la sincronización y su resultado estructurado

## 12. Milestone v1 exacto

### 12.1 Incluye

- scaffolding Tauri + Svelte + Tailwind
- commands backend base
- apertura y escaneo del workspace
- read/write/parse/serialize
- detección de workspace kind
- adapter Astro heurístico
- layout 3-panel
- editor funcional
- panel de metadatos para:
  - `blog`
  - `app-landings`
  - `projects`
- preview markdown
- preview mermaid
- placeholders para MDX Astro
- focus mode
- selector de fuente
- resolución segura de imágenes locales
- Git Sync manual con commit y push hacia el repositorio del workspace abierto
- PAT de GitHub en el almacén de credenciales del sistema operativo

### 12.2 No incluye

- parseo completo de `content.config.ts`
- upload o copia de imágenes
- drag & drop complejo
- renderer MDX completo
- export o publish
- Git Sync automático, en segundo plano o contra repositorios distintos del workspace abierto

### 12.3 Criterios de aceptación

- abre un workspace local de validación elegido por el usuario
- detecta si es Astro
- muestra colecciones reales
- abre un post real y guarda sin romper YAML
- abre una landing real y guarda sin romper YAML
- abre un project real y guarda sin romper YAML
- muestra preview markdown útil
- mermaid no rompe la app
- focus mode limpia la vista de forma evidente
- las imágenes locales del preview se resuelven sin salir del workspace root
- Git Sync se dispara manualmente, rechaza repositorios fuera del workspace y no recibe el PAT en su payload
- el PAT se conserva en el almacén de credenciales del sistema operativo y no en `localStorage`

## 13. Testing

### 13.1 Rust

- `parse_document` markdown con frontmatter
- `serialize_document` limpia `null`
- `read/write` validan la raíz
- `*.mmd` y `*.mermaid` se leen correctamente
- Git Sync rechaza una ruta de repositorio fuera de la raíz sin modificar el repositorio
- `commit_and_push` alcanza un remoto bare local: la prueba crea un remoto bare y un repositorio normal con identidad
  local, invoca el servicio real y comprueba que la rama remota recibió el commit con su contenido
- el servicio de secretos usa un store en memoria para las pruebas
- el probe del almacén de credenciales del sistema operativo (`#[ignore]`) escribe, lee y elimina una cuenta de prueba
  única; solo se ejecuta de forma explícita en un host, nunca en la suite normal ni en CI
- la resolución de imágenes rechaza destinos fuera de la raíz

### 13.2 Frontend

- `svelte-check` valida los contratos TypeScript y los componentes Svelte
- el build de producción valida el empaquetado frontend

### 13.3 CI

- el workflow `.github/workflows/ci.yml` se dispara en `push` y en `pull request`
- corre en las tres plataformas soportadas: `windows-latest`, `macos-latest` y `ubuntu-latest`
- toolchain: pnpm `11.18.0`, Node.js `22` y Rust `stable`, con caché de pnpm y de artefactos de Cargo
- por entrada de matriz ejecuta:
  - `pnpm install --frozen-lockfile`
  - `pnpm check`
  - `pnpm build`
  - `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- en Ubuntu instala antes de la prueba de Rust las dependencias de sistema de Tauri:
  `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `libxdo-dev`, `libssl-dev`, `librsvg2-dev` y `patchelf`
- no se empaquetan bundles de release en pull requests
- el plan y el estado de la primera ejecución están en `CI_PLAN.md`

Estado actual: el workflow está configurado, validado estructuralmente y ejecutado correctamente en Windows, macOS y Ubuntu.
La ejecución `42105342005` terminó con resultado correcto: https://github.com/naujrevilo/InterstellarWriterNext/actions/runs/42105342005

### 13.4 Verificaciones manuales pendientes

- recorrido completo de UI en Windows
- smoke tests de UI en macOS y Linux
- round-trip real en el almacén de credenciales de macOS y Linux (el de Windows ya se verificó en el host de la revisión)

## 14. Orden histórico de implementación

1. scaffold repo
2. commands + seguridad de paths
3. parser/serializer
4. apertura y escaneo del workspace + detección de kind
5. stores frontend
6. layout base
7. editor
8. metadata panel
9. preview markdown
10. mermaid
11. focus mode + fonts
12. imágenes locales
13. Git Sync y almacenamiento seguro del PAT

Este orden ya se ejecutó; se conserva como narrativa de construcción del prototipo.

## 15. Próximo paso de entrega

El alcance funcional de v1 está implementado y la CI ya está incorporada: el workflow `.github/workflows/ci.yml` está configurado, validado estructuralmente y terminó correctamente en la matriz Windows/macOS/Ubuntu de la ejecución `42105342005`.

La verificación determinista del camino de push ya existe y no depende de la red: una prueba del servicio de Git crea un remoto bare local y un repositorio normal con identidad local, invoca `commit_and_push` y comprueba que el commit llegó a la rama remota. Además, el servicio real publicó el commit `5149f20` en `origin/feature/stabilize-v1`, por lo que el envío autenticado a GitHub también quedó verificado.

El round-trip del almacén de credenciales del sistema operativo quedó verificado **solo en Windows**, en el host de esta revisión, con un probe ignorado que usa una cuenta de prueba única y siempre la elimina. Los round-trips de macOS y Linux siguen pendientes.

Quedan por separado los recorridos manuales de UI en las tres plataformas y el round-trip del almacén de credenciales en macOS y Linux; ninguno de esos pendientes contradice el alcance funcional descrito aquí.
