import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import electron from "vite-plugin-electron/simple";

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
                      // Native modules and Electron must NOT be bundled —
                      // Electron's Node loads them from node_modules at
                      // runtime via the asar's prebuilt .node binaries.
                      // Without this, Rolldown emits CJS `require("fs")`
                      // calls into an ESM bundle and the main process
                      // throws "Calling `require` for 'fs' in an
                      // environment that doesn't expose the `require`
                      // function" before reaching createWindow.
                      // See T-246.2 / T-243.19.
                      external: ["electron", "node-pty"],
                    },
                  },
                },
              },
              preload: {
                input: "electron/preload.ts",
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
