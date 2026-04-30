import { builtinModules } from "node:module";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import electron from "vite-plugin-electron/simple";

// Externals for the electron main + preload bundles. Native modules and
// Electron itself MUST NOT be bundled — Electron's Node loads them from
// node_modules at runtime via the asar's prebuilt .node binaries.
//
// Node built-ins must also be external because Vite v8/Rolldown otherwise
// emits CJS `require("fs")` calls into the ESM main bundle, and ESM in
// Electron's main process has no `require` — the app throws
// "Calling `require` for 'fs' in an environment that doesn't expose the
// `require` function" before reaching createWindow, helper processes
// never spawn, and no window appears.
//
// See T-243.19 (UAT-blocker symptom) and T-246.2 (root-cause family).
const electronMainExternal = [
  "electron",
  "node-pty",
  ...builtinModules,
  ...builtinModules.map((moduleName) => `node:${moduleName}`),
];

// https://vite.dev/config/
export default defineConfig(() => {
  const isElectronBuild =
    process.env.npm_lifecycle_event === "dev:electron" ||
    process.env.npm_lifecycle_event === "build:electron";

  return {
    plugins: [
      tailwindcss(),
      vue(),
      ...(isElectronBuild
        ? [
            electron({
              main: {
                entry: "electron/main.ts",
                vite: {
                  build: {
                    rollupOptions: {
                      external: electronMainExternal,
                    },
                  },
                },
              },
              preload: {
                input: "electron/preload.ts",
                vite: {
                  build: {
                    rollupOptions: {
                      external: electronMainExternal,
                    },
                  },
                },
              },
              renderer: {},
            }),
          ]
        : []),
    ],
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
    server: {
      host: "127.0.0.1",
    },
  };
});
