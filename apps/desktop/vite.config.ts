import { builtinModules } from "node:module";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import electron from "vite-plugin-electron/simple";

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
