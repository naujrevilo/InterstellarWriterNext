# CI Plan — GitHub Actions (Tauri v2 + Svelte)

> **Estado:** implementado en `.github/workflows/ci.yml`.
>
> El workflow está **configurado y validado estructuralmente** (parseo YAML y comprobación de comandos y matriz).
> La **primera ejecución alojada sigue pendiente**: el directorio `.git` de este workspace contiene únicamente la identidad de
> Engram, sin commits, referencias ni remoto, por lo que todavía no es un repositorio de trabajo listo para publicar.

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
| pnpm | `11.18.0` | `pnpm/action-setup@v4` con `version: 11.18.0` y `run_install: false` |
| Node.js | `22` | `actions/setup-node@v4` con `node-version: 22` y `cache: pnpm` |
| Rust | `stable` | `dtolnay/rust-toolchain@stable` |

Caché:

- **pnpm**: la caché de `actions/setup-node` (`cache: pnpm`) reutiliza el store de pnpm entre corridas.
- **Rust**: `Swatinem/rust-cache@v2` con `workspaces: src-tauri -> target` reutiliza los artefactos de compilación de Cargo.

## Pasos exactos del workflow

1. `actions/checkout@v4` — descarga el código del repositorio.
2. `pnpm/action-setup@v4` — instala pnpm `11.18.0`.
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

El workflow no se puede ejecutar de forma alojada desde este workspace (sin repositorio Git con remoto). La validación
realizada localmente fue:

- parseo del archivo YAML con un parser local y comprobación estructural de disparadores, matriz y comandos.
- `node_modules/.bin/svelte-check --tsconfig ./tsconfig.json` (equivalente local de `pnpm check`).
- `node_modules/.bin/vite build` (equivalente local de `pnpm build`).
- `cargo test --manifest-path src-tauri/Cargo.toml --locked` dentro del entorno de desarrollo de Visual Studio con
  `CC=cl` y `CXX=cl` (equivalente local del paso 10 en Windows).

## Pendientes para la primera ejecución alojada

- [ ] Publicar el código en un repositorio Git real con remoto configurado, para que GitHub Actions pueda disparar el workflow.
- [ ] Confirmar en la primera corrida que la instalación de paquetes de Linux, incluido `libayatana-appindicator3-dev`, resuelve en la imagen de Ubuntu del runner.
- [ ] Confirmar que `cargo test --locked` no reporta cambios pendientes en `src-tauri/Cargo.lock`.
- [ ] Verificar que la caché de pnpm y la de Rust se restauran correctamente en la segunda corrida.

## Fuera de alcance

- Empaquetado de release (`tauri build`) en pull requests.
- Pruebas de interfaz de usuario automatizadas.
- Round-trip del almacén de credenciales en macOS/Linux y push real a GitHub: siguen siendo verificaciones manuales.
