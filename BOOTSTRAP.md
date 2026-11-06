# Bootstrap — referencia para crear un prototipo nuevo

> Este documento es **material de bootstrap y referencia** para crear otro prototipo de Interstellar Writer Next. **No es el runbook actual de este repositorio.**
>
> El repositorio actual ya existe, usa `pnpm` y se ejecuta con los scripts definidos en `package.json`.

## 1. Propósito

Esta guía conserva la secuencia histórica para crear una carpeta de prototipo desde cero. Úsala solo cuando quieras iniciar una instancia nueva; para trabajar en el repositorio actual, consulta `package.json` y la documentación de arquitectura.

Destino de ejemplo:

`<parent-directory>/InterstellarWriterNext`

El placeholder se debe resolver en la máquina donde se cree el proyecto; no se debe copiar una ruta absoluta fija.

## 2. Requisitos

- Rust stable + Cargo
- Node.js LTS + pnpm
- Toolchain de Tauri para el sistema operativo

## 3. Crear el proyecto

Desde el directorio padre:

```powershell
pnpm create tauri-app InterstellarWriterNext
cd .\InterstellarWriterNext
```

Al configurar el template, selecciona:

- Frontend: Svelte
- Bundler: Vite
- Language: TypeScript

Instala las dependencias con el mismo gestor de paquetes del proyecto:

```powershell
pnpm install
```

## 4. Tailwind

Para la línea de Tailwind 3 usada por el prototipo actual:

```powershell
pnpm add -D tailwindcss@3 postcss autoprefixer
pnpm exec tailwindcss init -p
```

Configura Tailwind en Svelte/Vite según el template.

## 5. Correr en desarrollo

```powershell
pnpm tauri dev
```

## 6. Build

```powershell
pnpm tauri build
```

## 7. Copiar el paquete de referencia

Para un prototipo nuevo, copia o adapta desde la raíz del proyecto de referencia:

- `AGENTS.md`
- `MASTER_PROMPT.md`
- `SPEC_TECNICO_FINAL.md`
- `docs/` con las reglas, arquitectura, producto y punteros

Mantén los tres archivos de la raíz como fuentes canónicas y deja copias breves con enlaces en `docs/`. Actualiza el contenido en la raíz para evitar divergencias entre copias.
