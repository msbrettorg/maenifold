import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import electron from "vite-plugin-electron/simple";

// https://vite.dev/config/
export default defineConfig(() => {
  const isElectronBuild =
    process.env.npm_lifecycle_event === "dev:electron" ||
    process.env.npm_lifecycle_event === "build:electron";

  return {
    plugins: [
      vue(),
      ...(isElectronBuild
        ? [
            electron({
              main: {
                entry: "electron/main.ts",
              },
              preload: {
                input: "electron/preload.ts",
              },
              renderer: {},
            }),
          ]
        : []),
    ],
    server: {
      host: true,
    },
  };
});
