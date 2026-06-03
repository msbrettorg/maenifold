<script setup lang="ts">
// Vue Flow canvas host. Registers the custom node/edge types, wires the guarded
// Pinia⇄Vue Flow sync, and renders the notebook-grid background. Layout (ELK)
// and the reactivity-loop guard live in useCanvasSync; this component is the
// thin Vue Flow shell plus the empty-state and node-type palette legend.
import { computed, markRaw, onBeforeUnmount } from 'vue';
import { storeToRefs } from 'pinia';
import { VueFlow, useVueFlow } from '@vue-flow/core';
import type { Connection, EdgeMouseEvent, NodeMouseEvent } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Workflow } from '@lucide/vue';
import '@vue-flow/core/dist/style.css';
import '@vue-flow/core/dist/theme-default.css';
import { usePipelinesStore } from '../../stores/pipelines';
import { usePipelineDesignStore } from '../../stores/pipelineDesign';
import { useCanvasController } from './useCanvasController';
import { useCanvasSync } from './useCanvasSync';
import { useDesignController } from './design/useDesignController';
import { useSurgeryController } from './surgery/useSurgeryController';
import PipelineNode from './PipelineNode.vue';
import PipelineEdge from './PipelineEdge.vue';

const FLOW_ID = 'pipelines-canvas';

const store = usePipelinesStore();
const { selectedView, viewLoading } = storeToRefs(store);
const design = usePipelineDesignStore();
const controller = useCanvasController();
const designController = useDesignController();
const surgeryController = useSurgeryController();
const { layingOut, dispose } = useCanvasSync(FLOW_ID);
const { screenToFlowCoordinate } = useVueFlow(FLOW_ID);

const designModeActive = computed(() => design.designModeActive);

const nodeTypes = { pipeline: markRaw(PipelineNode) };
const edgeTypes = { pipeline: markRaw(PipelineEdge) };

const paletteEntries = [
  { type: 'prompt', label: 'Prompt', accent: 'var(--node-accent-prompt)' },
  { type: 'workflow', label: 'Workflow', accent: 'var(--node-accent-workflow)' },
  { type: 'sequentialThinking', label: 'Thinking', accent: 'var(--node-accent-thinking)' },
  { type: 'tool', label: 'Tool', accent: 'var(--node-accent-tool)' },
];

// --- design-mode context-menu + draw-to-connect wiring --------------------
// These handlers only act while design mode is active; otherwise the canvas is
// a read-only runtime view and native context menus are left alone.

function onPaneContextMenu(event: MouseEvent): void {
  if (!designModeActive.value) return;
  event.preventDefault();
  const flowPos = screenToFlowCoordinate({ x: event.clientX, y: event.clientY });
  designController.openPaneMenu(event.clientX, event.clientY, { x: flowPos.x, y: flowPos.y });
}

function onNodeContextMenu({ event, node }: NodeMouseEvent): void {
  const mouse = event as MouseEvent;
  if (designModeActive.value) {
    event.preventDefault();
    designController.openNodeMenu(mouse.clientX, mouse.clientY, node.id);
    return;
  }
  // Not in design mode: if a live (server-backed) pipeline is selected, open the
  // SURGERY menu for the right-clicked node. The two menus are mutually exclusive
  // (surgery is suppressed during design mode), so they never both open.
  if (!selectedView.value) return;
  event.preventDefault();
  surgeryController.openNodeMenu(mouse.clientX, mouse.clientY, node.id);
}

function onEdgeContextMenu({ event, edge }: EdgeMouseEvent): void {
  if (!designModeActive.value) return;
  event.preventDefault();
  const mouse = event as MouseEvent;
  designController.openEdgeMenu(mouse.clientX, mouse.clientY, edge.id);
}

function onConnect(connection: Connection): void {
  if (!designModeActive.value) return;
  const { source, target } = connection;
  if (!source || !target) return;
  const index = design.addEdge(source, target);
  if (index != null) {
    designController.openEdgeForm(index);
  }
}

onBeforeUnmount(dispose);
</script>

<template>
  <div class="canvas-host">
    <VueFlow
      :id="FLOW_ID"
      :node-types="nodeTypes"
      :edge-types="edgeTypes"
      :default-viewport="{ zoom: 0.85, x: 0, y: 0 }"
      :min-zoom="0.2"
      :max-zoom="2"
      :snap-to-grid="true"
      :snap-grid="[16, 16]"
      :elevate-edges-on-select="true"
      fit-view-on-init
      @pane-context-menu="onPaneContextMenu"
      @node-context-menu="onNodeContextMenu"
      @edge-context-menu="onEdgeContextMenu"
      @connect="onConnect"
    >
      <Background :gap="28" :size="1" pattern-color="var(--border-color)" />
    </VueFlow>

    <div v-if="layingOut" class="canvas-status" role="status">Laying out…</div>

    <div
      v-if="!selectedView && !viewLoading"
      class="canvas-empty"
    >
      <Workflow :size="30" :stroke-width="1.4" />
      <p>Select a pipeline to render its graph.</p>
    </div>

    <aside v-if="controller.paletteOpen.value" class="canvas-palette" aria-label="Node type palette">
      <span class="palette-title">Node types</span>
      <ul>
        <li v-for="entry in paletteEntries" :key="entry.type">
          <span class="palette-swatch" :style="{ background: entry.accent }" />
          {{ entry.label }}
        </li>
      </ul>
      <span class="palette-hint">⚡ tool nodes spend 0 tokens</span>
    </aside>
  </div>
</template>

<style scoped>
.canvas-host {
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.canvas-host :deep(.vue-flow) {
  background: transparent;
}

.canvas-status {
  position: absolute;
  top: 0.6rem;
  left: 50%;
  transform: translateX(-50%);
  padding: 0.25rem 0.7rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-main) 88%, var(--bg-sidebar));
  color: var(--text-secondary);
  font-size: 0.72rem;
  z-index: 5;
}

.canvas-empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  color: var(--text-muted);
  pointer-events: none;
}

.canvas-empty p {
  margin: 0;
  font-size: 0.85rem;
}

.canvas-palette {
  position: absolute;
  top: 0.7rem;
  right: 0.7rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.6rem 0.7rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.12);
  z-index: 6;
}

.canvas-palette .palette-title {
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-secondary);
}

.canvas-palette ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.canvas-palette li {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.78rem;
  color: var(--text-primary);
}

.palette-swatch {
  width: 12px;
  height: 12px;
  border-radius: 3px;
}

.palette-hint {
  margin-top: 0.15rem;
  font-size: 0.68rem;
  color: var(--text-muted);
}
</style>
