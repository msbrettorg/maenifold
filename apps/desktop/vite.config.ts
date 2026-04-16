import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import electron from "vite-plugin-electron/simple";

const pipelineComposerShellSourcePrefixes = [
  "/src/components/PipelineComposer.vue",
  "/src/components/PipelineComposerContextMenu",
  "/src/components/PipelineComposerToolbar",
  "/src/composables/useLayout",
];

const pipelineComposerCanvasLogicPrefixes = [
  "/src/composables/useComposerCanvas",
  "/src/composables/useComposerSync",
  "/src/lib/pipeline-composer-flow",
];

const pipelineComposerInteractionLogicPrefixes = [
  "/src/composables/useComposerContextMenu",
  "/src/composables/useComposerKeyboard",
  "/src/composables/useComposerPalette",
  "/src/composables/useInspectorActions",
  "/src/lib/pipeline-composer-edge-utils",
  "/src/lib/pipeline-composer-node-actions",
];

const pipelineNodeCardSourcePrefixes = [
  "/src/components/PipelineNodeCard",
  "/src/lib/pipeline-node",
];

const pipelinePaletteSourcePrefixes = [
  "/src/components/AddNode",
  "/src/components/EdgeMappingEditor",
  "/src/components/ToolNodeFields",
  "/src/components/ToolPaletteFields",
  "/src/composables/useAddNodeForm",
  "/src/composables/useAddNodeFormFieldBindings",
  "/src/composables/useToolPaletteFields",
  "/src/lib/tool-palette-catalog",
  "/src/stores/node-palette",
];

const pipelineConfigDialogSourcePrefixes = [
  "/src/components/DesignNodeConfig",
  "/src/components/PromptAssemblyPreview",
  "/src/components/SchemaForm",
];

const pipelineAuxiliaryDialogSourcePrefixes = [
  "/src/components/GraftNodesDialog",
  "/src/components/JoinPipelineDialog",
  "/src/components/PromotionDialog",
  "/src/components/SemanticZoomOverlay",
  "/src/components/TemplateGallery",
  "/src/composables/useGraftNodes",
  "/src/composables/useMemoryPromotion",
];

const pipelineRuntimeSourcePrefixes = [
  "/src/components/PipelineActionBar",
  "/src/components/PipelineFlyout",
  "/src/components/PipelineIconRail",
  "/src/components/PipelineRunner",
  "/src/components/PipelineSidebar",
  "/src/composables/useFlyout",
  "/src/composables/usePipelineDialogs",
  "/src/composables/usePipelineLoading",
  "/src/composables/usePipelineMobile",
];

const pipelineStateSourcePrefixes = [
  "/src/composables/pipeline-",
  "/src/composables/usePipelineMcp",
  "/src/lib/pipeline-",
  "/src/stores/pipeline",
  "/src/stores/pipeline-",
  "/src/stores/semantic-zoom",
];

function includesAny(id: string, prefixes: string[]) {
  return prefixes.some((prefix) => id.includes(prefix));
}

function manualChunks(id: string) {
  const normalizedId = id.replaceAll("\\", "/");

  if (includesAny(normalizedId, pipelineComposerShellSourcePrefixes)) {
    return "pipeline-composer-shell-source";
  }

  if (includesAny(normalizedId, pipelineComposerCanvasLogicPrefixes)) {
    return "pipeline-composer-canvas-logic";
  }

  if (includesAny(normalizedId, pipelineComposerInteractionLogicPrefixes)) {
    return "pipeline-composer-interaction-logic";
  }

  if (includesAny(normalizedId, pipelineNodeCardSourcePrefixes)) {
    return "pipeline-node-card-source";
  }

  if (includesAny(normalizedId, pipelinePaletteSourcePrefixes)) {
    return "pipeline-palette-source";
  }

  if (includesAny(normalizedId, pipelineConfigDialogSourcePrefixes)) {
    return "pipeline-config-dialog-source";
  }

  if (includesAny(normalizedId, pipelineAuxiliaryDialogSourcePrefixes)) {
    return "pipeline-aux-dialog-source";
  }

  if (includesAny(normalizedId, pipelineRuntimeSourcePrefixes)) {
    return "pipeline-runtime-source";
  }

  if (includesAny(normalizedId, pipelineStateSourcePrefixes)) {
    return "pipeline-state-source";
  }

  if (normalizedId.includes("/node_modules/@vue-flow/")) {
    return "pipeline-vueflow-vendor";
  }

  if (normalizedId.includes("/node_modules/elkjs/")) {
    return "pipeline-elk-vendor";
  }

  if (
    normalizedId.includes("/node_modules/@jsonforms/core/") ||
    normalizedId.includes("/node_modules/@jsonforms/vue/") ||
    normalizedId.includes("/node_modules/@jsonforms/vue-vanilla/")
  ) {
    return "pipeline-form-vendor";
  }

  return undefined;
}

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
      host: true,
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks,
        },
      },
    },
  };
});
