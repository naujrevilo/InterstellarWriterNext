# CI Plan — GitHub Actions (Tauri v2 + Svelte)

> **Estado:** implementado en `.github/workflows/ci.yml`.
>
> El workflow está **configurado, validado estructuralmente y ejecutado correctamente** en Windows, macOS y Ubuntu.
> Evidencia: [ejecución 42105342005](https://github.com/naujrevilo/InterstellarWriterNext/actions/runs/42105342005).

## Objetivo

Validar en cada `push` y en cada `pull request`:

- instalación reproducible de las dependencias del frontend con pnpm
- verificación de tipos del frontend (`pnpm check`)
- build de producción del frontend (`pnpm build`)
- pruebas unitarias del backend Rust (`cargo test`)

**No** se construyen bundles de release en pull requests. Un job de empaquetado por plataforma queda fuera del alcance de v1.

## Matriz

Un único job (`ci`) corre en las tres plataformas soportadas:

| Sistema | Runner | Alcance |
| --- | --- | --- |
| Windows | `windows-latest` | toolchain MSVC preinstalado en el runner |
| macOS | `macos-latest` | Xcode Command Line Tools preinstaladas en el runner |
| Ubuntu | `ubuntu-latest` | requiere dependencias de sistema de Tauri |

## Toolchain

| Componente | Versión | Cómo se instala |
| --- | --- | --- |
| pnpm | `11.18.0` | `pnpm/action-setup@v4` toma la versión declarada en `package.json` (`packageManager`) y usa `run_install: false` |
| Node.js | `22` | `actions/setup-node@v4` con `node-version: 22` y `cache: pnpm` |
| Rust | `stable` | `dtolnay/rust-toolchain@stable` |

Caché:

- **pnpm**: la caché de `actions/setup-node` (`cache: pnpm`) reutiliza el store de pnpm entre corridas.
- **Rust**: `Swatinem/rust-cache@v2` con `workspaces: src-tauri -> target` reutiliza los artefactos de compilación de Cargo.

## Pasos exactos del workflow

1. `actions/checkout@v4` — descarga el código del repositorio.
2. `pnpm/action-setup@v4` — instala la versión de pnpm `11.18.0` declarada en `package.json`.
3. `actions/setup-node@v4` — instala Node.js `22` y activa la caché de pnpm.
4. `pnpm install --frozen-lockfile` — instala dependencias sin modificar `pnpm-lock.yaml`.
5. `pnpm check` — ejecuta `svelte-check --tsconfig ./tsconfig.json`.
6. `pnpm build` — ejecuta `vite build`.
7. Dependencias de sistema de Linux (solo `runner.os == 'Linux'`), antes de la prueba de Rust:
   ```
   sudo apt-get update
   sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libxdo-dev libssl-dev librsvg2-dev patchelf
   ```
8. `dtolnay/rust-toolchain@stable` — instala Rust estable.
9. `Swatinem/rust-cache@v2` — cachea los artefactos de `src-tauri/target`.
10. `cargo test --manifest-path src-tauri/Cargo.toml --locked` — pruebas unitarias del backend Rust.

Los pasos 2 a 6 y el paso 10 se ejecutan en **cada** entrada de la matriz. El paso 7 solo se ejecuta en Ubuntu.

## Verificación local equivalente

La validación local equivalente fue:

- parseo del archivo YAML con un parser local y comprobación estructural de disparadores, matriz y comandos.
- `node_modules/.bin/svelte-check --tsconfig ./tsconfig.json` (equivalente local de `pnpm check`).
- `node_modules/.bin/vite build` (equivalente local de `pnpm build`).
- `cargo test --manifest-path src-tauri/Cargo.toml --locked` dentro del entorno de desarrollo de Visual Studio con
  `CC=cl` y `CXX=cl` (equivalente local del paso 10 en Windows).

## Estado de la ejecución alojada

- [x] Código publicado en `https://github.com/naujrevilo/InterstellarWriterNext` con remoto configurado.
- [x] Dependencias de Linux, incluido `libayatana-appindicator3-dev`, instaladas correctamente en `ubuntu-latest`.
- [x] `cargo test --locked` completado sin modificar `src-tauri/Cargo.lock`.
- [x] Cachés de pnpm y Rust restauradas correctamente en ejecuciones posteriores.
- [x] Matriz completa correcta: `windows-latest`, `macos-latest` y `ubuntu-latest`.

## Fuera de alcance

- Empaquetado de release (`tauri build`) en pull requests.
- Pruebas de interfaz de usuario automatizadas.
- Round-trip del almacén de credenciales en macOS/Linux: sigue siendo una verificación manual.
