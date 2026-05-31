import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Read the package version once at config-evaluation time so we can inject
// it into the web build (the Tauri build reads it from `tauri.conf.json`
// instead via `@tauri-apps/api/app#getVersion`).
const pkg = JSON.parse(
  readFileSync(fileURLToPath(new URL("./package.json", import.meta.url)), "utf-8")
) as { version: string };

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  const isWeb = mode === "web";

  return {
    plugins: [vue()],

    define: {
      // Exposed to the frontend as `import.meta.env.VITE_APP_VERSION`. The
      // host abstraction (`src/lib/host/index.ts`) reads this on the web
      // build and falls back to it when `@tauri-apps/api/app` is unavailable.
      "import.meta.env.VITE_APP_VERSION": JSON.stringify(pkg.version),
    },

    // Web builds emit to `dist-web/` so the Tauri build pipeline (which
    // expects `frontendDist: ../dist`) is unaffected.
    build: isWeb
      ? {
          outDir: "dist-web",
          emptyOutDir: true,
        }
      : undefined,

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: isWeb
      ? {
          // Browser dev server: use Vite defaults so it works behind common
          // proxies / Dev Tunnels without the strict-port behaviour Tauri
          // requires.
          port: 5173,
        }
      : {
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
            // 3. tell Vite to ignore watching `src-tauri`
            ignored: ["**/src-tauri/**"],
          },
        },
  };
});
