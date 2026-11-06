import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

// Vite/Rollup aggressively tree-shakes `import "..."` statements that
// look unused (the import has no binding). Prism's language grammars
// are IIFEs that mutate `Prism.languages` and have no exports, so
// the bundler drops them. Listing them under `optimizeDeps.include`
// forces Vite to pre-bundle them and keeps them in the final output.
const PRISM_GRAMMARS = [
  "prismjs/components/prism-clike",
  "prismjs/components/prism-javascript",
  "prismjs/components/prism-typescript",
  "prismjs/components/prism-jsx",
  "prismjs/components/prism-tsx",
  "prismjs/components/prism-rust",
  "prismjs/components/prism-python",
  "prismjs/components/prism-bash",
  "prismjs/components/prism-powershell",
  "prismjs/components/prism-json",
  "prismjs/components/prism-yaml",
  "prismjs/components/prism-css",
  "prismjs/components/prism-markup",
  "prismjs/components/prism-sql",
  "prismjs/components/prism-go",
  "prismjs/components/prism-java",
  "prismjs/components/prism-c",
  "prismjs/components/prism-cpp",
  "prismjs/components/prism-markdown",
  "prismjs/components/prism-toml",
  "prismjs/components/prism-diff",
  "prismjs/components/prism-docker",
  "prismjs/components/prism-ruby",
  "prismjs/components/prism-swift",
  "prismjs/components/prism-kotlin",
  "prismjs/components/prism-perl",
  "prismjs/components/prism-r",
  "prismjs/components/prism-scala",
  "prismjs/components/prism-lua",
  "prismjs/components/prism-haskell",
];

export default defineConfig(async () => ({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  optimizeDeps: {
    include: ["prismjs", ...PRISM_GRAMMARS],
  },
  build: {
    rollupOptions: {
      // Same problem on the build side: grammars get tree-shaken.
      // Listing them here keeps them in the final bundle.
      external: [],
    },
  },
}));
