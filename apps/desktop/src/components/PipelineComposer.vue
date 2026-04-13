<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { BaseEdge, EdgeLabelRenderer, VueFlow } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Controls } from '@vue-flow/controls';
import { MiniMap } from '@vue-flow/minimap';

import '@vue-flow/core/dist/style.css';
import '@vue-flow/core/dist/theme-default.css';
import '@vue-flow/controls/dist/style.css';
import '@vue-flow/minimap/dist/style.css';

import AddNodeForm from './AddNodeForm.vue';
import PipelineComposerContextMenu from './PipelineComposerContextMenu.vue';
import { useComposerCanvas } from '../composables/useComposerCanvas';
import { useMcpToolDiscovery } from '../composables/useMcpToolDiscovery';
import { usePipelineMobile } from '../composables/usePipelineMobile';
import { useResizeHandle } from '../composables/useResizeHandle';
import type {
  PipelineNode,
  PipelineDefinition,
  PipelineNodeType,
  PipelineView,
} from '../lib/pipeline-types';
import {
  createNode,
  createNodeId,
  type PipelineNodeDraftByType,
  type PipelineNodeInputByType,
} from '../lib/pipeline-node-factory';
import { getBareToolName } from '../lib/tool-categories';
import { resolvePreferredAgent } from '../lib/preferred-agent';
import { NODE_PALETTE_MIN_WIDTH, useNodePaletteStore } from '../stores/node-palette';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{
  designMode: boolean;
  localDefinition?: PipelineDefinition;
  view?: PipelineView | null;
}>();

const emit = defineEmits<{
  (e: 'update:localDefinition', value: PipelineDefinition): void;
  (e: 'nodeSelected', nodeId: string | null): void;
  (e: 'nodeDoubleClick', nodeId: string): void;
  (e: 'edgeSelected', value: { source: string; target: string } | null): void;
  (e: 'openGraft'): void;
  (e: 'openPromotion'): void;
}>();

const {
  createNodeAt,
  defaultEdgeOptions,
  deleteSelected,
  deleteNode,
  disconnectNodeEdges,
  duplicateNode,
  edgeIndicator,
  edgePath,
  edgeTooltip,
  editNode,
  edges,
  fitView,
  handleAutoLayout,
  handleFlowKeyDown,
  hasSelection,
  nodeTypes,
  nodes,
  showEmptyState,
  toFlowPosition,
} = useComposerCanvas({
  designMode: () => props.designMode,
  localDefinition: () => props.localDefinition,
  view: () => props.view,
  updateDefinition: (definition) => emit('update:localDefinition', definition),
  selectNode: (nodeId) => emit('nodeSelected', nodeId),
  selectEdge: (edge) => emit('edgeSelected', edge),
  designNodeDoubleClick: (nodeId) => emit('nodeDoubleClick', nodeId),
});

const store = usePipelineStore();
const configStore = useConfigStore();
const nodePaletteStore = useNodePaletteStore();
const { serverResults, hasLoadedCatalog, refreshTools } = useMcpToolDiscovery();
const paletteResize = useResizeHandle({
  initial: nodePaletteStore.panelWidth,
  min: NODE_PALETTE_MIN_WIDTH,
  max: 720,
});

const paletteOpen = ref(false);
const { isNarrowViewport } = usePipelineMobile();

const paletteAsOverlay = computed(() => isNarrowViewport.value && paletteOpen.value);

const designGridStyle = computed(() => ({
  'grid-template-columns': !paletteOpen.value || isNarrowViewport.value
    ? 'minmax(0, 1fr)'
    : `${paletteResize.width.value}px minmax(0, 1fr)`,
}));

watch(
  () => paletteResize.width.value,
  (value) => {
    nodePaletteStore.setPanelWidth(value);
  },
  { immediate: true },
);

const NODE_TYPE_COLORS: Record<string, string> = {
  workflow: '#4d8eff',
  prompt: '#adc6ff',
  sequentialThinking: '#d0bcff',
  tool: '#4edea3',
};

function miniMapNodeColor(node: { data?: { type?: string } }): string {
  return NODE_TYPE_COLORS[node.data?.type ?? ''] ?? NODE_TYPE_COLORS.workflow;
}

function submitNode<T extends PipelineNodeType>(
  type: T,
  fields: PipelineNodeDraftByType[T],
): void {
  const node = store.addNode(
    createNode(type, { id: createNodeId(type), ...fields } as PipelineNodeInputByType[T]),
  );
  void nextTick(() => {
    editNode(node.id);
    fitView();
  });
}

type ContextMenuState =
  | {
      mode: 'canvas';
      x: number;
      y: number;
      position: { x: number; y: number };
    }
  | {
      mode: 'node';
      x: number;
      y: number;
      nodeId: string;
    };

const contextMenu = ref<ContextMenuState | null>(null);

const firstAgent = computed(() => resolvePreferredAgent(configStore.agentNames));
const firstWorkflow = computed(() => store.workflows[0] ?? '');
const toolGroups = computed(() =>
  serverResults.value
    .map((result) => ({
      server: result.server,
      tools: [...result.tools]
        .sort((left, right) => left.name.localeCompare(right.name))
        .map((tool) => ({
          name: tool.name,
          displayName: tool.name.startsWith(`${result.server}.`)
            ? tool.name.slice(result.server.length + 1)
            : getBareToolName(tool.name),
          description: tool.description,
        })),
    }))
    .filter((group) => group.tools.length > 0),
);

function closeContextMenu(): void {
  contextMenu.value = null;
}

function createNodeAtPosition(node: PipelineNode, position: { x: number; y: number }): void {
  createNodeAt(node, position);
  closeContextMenu();
}

function maybeRefreshComposerCatalogs(): void {
  if (!props.designMode) return;
  if (store.workflows.length === 0) {
    void store.refreshWorkflows();
  }
  if (!hasLoadedCatalog.value) {
    void refreshTools();
  }
}

function handleComposerContextMenu(event: MouseEvent): void {
  event.preventDefault();

  const target = event.target;
  if (!(target instanceof Element)) {
    return;
  }

  if (target.closest('.composer-context-menu, .vue-flow__controls, .vue-flow__minimap')) {
    return;
  }

  const currentTarget = event.currentTarget;
  const flowRoot = currentTarget instanceof HTMLElement
    ? currentTarget.querySelector<HTMLElement>('.vue-flow')
    : target.closest<HTMLElement>('.vue-flow');
  if (!flowRoot) {
    closeContextMenu();
    return;
  }

  const nodeElement = target.closest<HTMLElement>('.vue-flow__node');
  const nodeId = nodeElement?.dataset.id;
  if (nodeId) {
    editNode(nodeId);
    contextMenu.value = {
      mode: 'node',
      x: event.clientX,
      y: event.clientY,
      nodeId,
    };
    return;
  }

  if (target.closest('.vue-flow__edge, .edge-indicator')) {
    closeContextMenu();
    return;
  }

  if (!props.designMode) {
    contextMenu.value = {
      mode: 'canvas',
      x: event.clientX,
      y: event.clientY,
      position: toFlowPosition(event.clientX, event.clientY, flowRoot),
    };
    return;
  }

  contextMenu.value = {
    mode: 'canvas',
    x: event.clientX,
    y: event.clientY,
    position: toFlowPosition(event.clientX, event.clientY, flowRoot),
  };
}

function handleWindowPointerDown(event: PointerEvent): void {
  const target = event.target;
  if (!(target instanceof Element)) {
    closeContextMenu();
    return;
  }

  if (!target.closest('.composer-context-menu')) {
    closeContextMenu();
  }
}

function handleWindowKeyDown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    closeContextMenu();
  }
}

function createPromptNode(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'canvas' || !firstAgent.value) return;

  createNodeAtPosition(
    createNode('prompt', {
      id: createNodeId('prompt'),
      agent: firstAgent.value,
      prompt: 'Ask an agent',
    }),
    contextMenu.value.position,
  );
}

function createThinkingNode(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'canvas' || !firstAgent.value) return;

  createNodeAtPosition(
    createNode('sequentialThinking', {
      id: createNodeId('sequentialThinking'),
      agent: firstAgent.value,
      topic: 'Think through a problem',
      totalThoughts: 5,
    }),
    contextMenu.value.position,
  );
}

function createWorkflowNode(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'canvas' || !firstWorkflow.value) return;

  createNodeAtPosition(
    createNode('workflow', {
      id: createNodeId('workflow'),
      workflowId: firstWorkflow.value,
      agent: '',
    }),
    contextMenu.value.position,
  );
}

function createToolNode(toolName: string): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'canvas') return;

  createNodeAtPosition(
    createNode('tool', {
      id: createNodeId('tool'),
      toolName,
    }),
    contextMenu.value.position,
  );
}

function handleEditNode(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
  emit('nodeDoubleClick', contextMenu.value.nodeId);
  closeContextMenu();
}

function handleDuplicateNode(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
  duplicateNode(contextMenu.value.nodeId);
  closeContextMenu();
}

function handleDeleteNode(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
  deleteNode(contextMenu.value.nodeId);
  closeContextMenu();
}

function handleDisconnectEdges(): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
  disconnectNodeEdges(contextMenu.value.nodeId);
  closeContextMenu();
}

function handleServerNodeAction(action: 'restartNode' | 'rewindNode' | 'forkNode' | 'abandonNode'): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
  const { nodeId } = contextMenu.value;
  closeContextMenu();
  switch (action) {
    case 'restartNode':
      void store.restartNode(nodeId);
      break;
    case 'rewindNode':
      void store.rewindNodes([nodeId]);
      break;
    case 'forkNode': {
      if (!store.currentPipelineId) break;
      const nameInput = window.prompt('Optional name for the forked pipeline session:', '');
      if (nameInput === null) break;
      const forkName = nameInput.trim() || undefined;
      void store.forkPipeline(store.currentPipelineId, nodeId, forkName);
      break;
    }
    case 'abandonNode':
      void store.updateNodeStatus(nodeId, 'abandoned');
      break;
  }
}

const currentPipelineStatus = computed(() => props.view?.status ?? null);
const currentPipelineRunning = computed(() => props.view?.running ?? false);
const hasRewindableNodes = computed(() =>
  props.view?.nodes.some((n) => n.status === 'completed' || n.status === 'abandoned' || n.status === 'stuck') ?? false,
);
const hasCompletedNodesWithOutput = computed(() =>
  props.view?.nodes.some((n) => n.status === 'completed' && Boolean(n.output)) ?? false,
);

const contextMenuNodeStatus = computed(() => {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return null;
  const nodeId = contextMenu.value.nodeId;
  return props.view?.nodes.find((n) => n.nodeId === nodeId)?.status ?? null;
});

const contextMenuNodeHasOutput = computed(() => {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return false;
  const nodeId = contextMenu.value.nodeId;
  const viewNode = props.view?.nodes.find((n) => n.nodeId === nodeId);
  return Boolean(viewNode?.output);
});

function handleCanvasRunPipeline(): void {
  void store.runPipeline();
  closeContextMenu();
}

function handleCanvasRewindAll(): void {
  const rewindableIds = props.view?.nodes
    .filter((n) => n.status === 'completed' || n.status === 'abandoned' || n.status === 'stuck')
    .map((n) => n.nodeId) ?? [];
  if (rewindableIds.length > 0) {
    void store.rewindNodes(rewindableIds);
  }
  closeContextMenu();
}

function handleCanvasGraft(): void {
  emit('openGraft');
  closeContextMenu();
}

function handleCanvasPromoteToMemory(): void {
  emit('openPromotion');
  closeContextMenu();
}

watch(() => props.designMode, maybeRefreshComposerCatalogs, { immediate: true });

onMounted(() => {
  window.addEventListener('pointerdown', handleWindowPointerDown);
  window.addEventListener('keydown', handleWindowKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', handleWindowPointerDown);
  window.removeEventListener('keydown', handleWindowKeyDown);
});

defineExpose({ deleteSelected, fitView, handleFlowKeyDown });
</script>

<template>
  <div class="pipeline-composer" @keydown="handleFlowKeyDown">
    <div v-if="designMode" class="composer-layout" :style="designGridStyle">
      <section v-if="paletteOpen && !paletteAsOverlay" class="composer-palette">
        <AddNodeForm
          :workflows="store.workflows"
          :submit-node="submitNode"
        />
        <div
          class="composer-palette-resize"
          :style="{ left: `${paletteResize.width.value}px` }"
          @mousedown="paletteResize.startResize($event, 'right')"
        />
      </section>

      <section class="composer-canvas" @contextmenu.capture="handleComposerContextMenu">
        <div class="flow-toolbar">
          <button type="button" class="btn btn-ghost" @click="handleAutoLayout('LR')">↔ Layout LR</button>
          <button type="button" class="btn btn-ghost" @click="handleAutoLayout('TB')">↕ Layout TB</button>
          <button v-if="hasSelection" type="button" class="btn btn-outline-danger" @click="deleteSelected()">
            Delete Selected
          </button>
          <button type="button" class="btn btn-ghost" @click="fitView()">⊞ Fit View</button>
          <button
            type="button"
            class="btn btn-ghost composer-palette-toggle"
            :aria-pressed="paletteOpen"
            @click="paletteOpen = !paletteOpen"
          >
            {{ paletteOpen ? '✕ Palette' : '+ Palette' }}
          </button>
          <slot name="toolbar-actions" />
        </div>

        <Transition name="palette-overlay">
          <section v-if="paletteAsOverlay" class="composer-palette-overlay">
            <div class="composer-palette-overlay-header">
              <strong>Add Node</strong>
              <button type="button" class="btn btn-ghost btn-xs" @click="paletteOpen = false">✕ Close</button>
            </div>
            <AddNodeForm
              :workflows="store.workflows"
              :submit-node="submitNode"
            />
          </section>
        </Transition>
        <button
          v-if="paletteAsOverlay"
          type="button"
          class="composer-palette-backdrop"
          aria-label="Close palette"
          @click="paletteOpen = false"
        />
        <VueFlow
          v-model:nodes="nodes"
          v-model:edges="edges"
          :node-types="nodeTypes"
          :default-edge-options="defaultEdgeOptions"
          :default-viewport="{ zoom: 1, x: 0, y: 0 }"
          :min-zoom="0.2"
          :max-zoom="2"
          :snap-to-grid="true"
          :snap-grid="[20, 20]"
          :nodes-draggable="designMode"
          :nodes-connectable="designMode"
          :edges-updatable="designMode"
          :elements-selectable="designMode"
          :delete-key-code="designMode ? 'Backspace' : null"
          fit-view-on-init
          class="pipeline-flow"
        >
          <Background color="#2a2a2a" />
          <Controls :show-interactive="designMode" />
          <MiniMap :node-color="miniMapNodeColor" mask-color="rgba(0,0,0,0.25)" />
          <template v-slot:edge-smoothstep="edgeProps">
            <g>
              <BaseEdge
                :id="edgeProps.id"
                :path="edgePath(edgeProps).d"
                :style="edgeProps.style"
                :marker-start="edgeProps.markerStart"
                :marker-end="edgeProps.markerEnd"
                :interaction-width="edgeProps.interactionWidth"
              />
              <title>{{ edgeTooltip(edgeProps.data) }}</title>
            </g>
            <EdgeLabelRenderer v-if="edgeIndicator(edgeProps)">
              <div
                :class="['edge-indicator', `edge-indicator--${edgeIndicator(edgeProps)}`]"
                :style="{ transform: `translate(-50%, -50%) translate(${edgePath(edgeProps).x}px, ${edgePath(edgeProps).y}px)` }"
                :title="edgeTooltip(edgeProps.data)"
              >
                {{ edgeIndicator(edgeProps) === 'mapped' ? '⇄' : '⚠' }}
              </div>
            </EdgeLabelRenderer>
          </template>
        </VueFlow>
        <div v-if="showEmptyState" class="composer-empty-state">
          <span>Add nodes using the form or right-click the canvas</span>
        </div>
        <PipelineComposerContextMenu
          v-if="contextMenu"
          :x="contextMenu.x"
          :y="contextMenu.y"
          :mode="designMode ? 'design' : 'server'"
          :target="contextMenu.mode"
          :tool-groups="toolGroups"
          :can-ask-agent="Boolean(firstAgent)"
          :can-think="Boolean(firstAgent)"
          :can-run-workflow="Boolean(firstWorkflow)"
          @close="closeContextMenu"
          @ask-agent="createPromptNode"
          @think-problem="createThinkingNode"
          @run-workflow="createWorkflowNode"
          @run-tool="createToolNode"
          @edit="handleEditNode"
          @duplicate="handleDuplicateNode"
          @delete="handleDeleteNode"
          @disconnect-edges="handleDisconnectEdges"
          @restart="handleServerNodeAction('restartNode')"
          @rewind="handleServerNodeAction('rewindNode')"
          @fork="handleServerNodeAction('forkNode')"
          @abandon="handleServerNodeAction('abandonNode')"
        />
      </section>
    </div>

    <template v-else>
      <div class="flow-toolbar">
        <button type="button" class="btn btn-ghost" @click="handleAutoLayout('LR')">↔ Layout LR</button>
        <button type="button" class="btn btn-ghost" @click="handleAutoLayout('TB')">↕ Layout TB</button>
        <slot name="toolbar-actions" />
      </div>
      <section class="composer-canvas" @contextmenu.capture="handleComposerContextMenu">
        <VueFlow
          v-model:nodes="nodes"
          v-model:edges="edges"
          :node-types="nodeTypes"
          :default-edge-options="defaultEdgeOptions"
          :default-viewport="{ zoom: 1, x: 0, y: 0 }"
          :min-zoom="0.2"
          :max-zoom="2"
          :snap-to-grid="true"
          :snap-grid="[20, 20]"
          :nodes-draggable="designMode"
          :nodes-connectable="designMode"
          :edges-updatable="designMode"
          :elements-selectable="true"
          :delete-key-code="designMode ? 'Backspace' : null"
          class="pipeline-flow"
        >
          <Background color="#2a2a2a" />
          <Controls :show-interactive="designMode" />
          <MiniMap :node-color="miniMapNodeColor" mask-color="rgba(0,0,0,0.25)" />
          <template v-slot:edge-smoothstep="edgeProps">
            <g>
              <BaseEdge
                :id="edgeProps.id"
                :path="edgePath(edgeProps).d"
                :style="edgeProps.style"
                :marker-start="edgeProps.markerStart"
                :marker-end="edgeProps.markerEnd"
                :interaction-width="edgeProps.interactionWidth"
              />
              <title>{{ edgeTooltip(edgeProps.data) }}</title>
            </g>
            <EdgeLabelRenderer v-if="edgeIndicator(edgeProps)">
              <div
                :class="['edge-indicator', `edge-indicator--${edgeIndicator(edgeProps)}`]"
                :style="{ transform: `translate(-50%, -50%) translate(${edgePath(edgeProps).x}px, ${edgePath(edgeProps).y}px)` }"
                :title="edgeTooltip(edgeProps.data)"
              >
                {{ edgeIndicator(edgeProps) === 'mapped' ? '⇄' : '⚠' }}
              </div>
            </EdgeLabelRenderer>
          </template>
        </VueFlow>
        <div v-if="showEmptyState" class="composer-empty-state">
          <span>Add your first node</span>
          <span class="composer-empty-state-arrow" aria-hidden="true">↖</span>
        </div>
        <PipelineComposerContextMenu
          v-if="contextMenu"
          :x="contextMenu.x"
          :y="contextMenu.y"
          mode="server"
          :target="contextMenu.mode"
          :tool-groups="toolGroups"
          :can-ask-agent="Boolean(firstAgent)"
          :can-think="Boolean(firstAgent)"
          :can-run-workflow="Boolean(firstWorkflow)"
          :pipeline-status="currentPipelineStatus"
          :pipeline-running="currentPipelineRunning"
          :has-rewindable-nodes="hasRewindableNodes"
          :has-completed-nodes-with-output="hasCompletedNodesWithOutput"
          :node-status="contextMenuNodeStatus"
          :node-has-output="contextMenuNodeHasOutput"
          @close="closeContextMenu"
          @restart="handleServerNodeAction('restartNode')"
          @rewind="handleServerNodeAction('rewindNode')"
          @fork="handleServerNodeAction('forkNode')"
          @abandon="handleServerNodeAction('abandonNode')"
          @run-pipeline="handleCanvasRunPipeline"
          @rewind-all="handleCanvasRewindAll"
          @graft="handleCanvasGraft"
          @promote-to-memory="handleCanvasPromoteToMemory"
        />
      </section>
    </template>
  </div>
</template>

<style scoped>
.pipeline-composer {
  width: 100%;
  height: 100%;
  min-height: 400px;
  position: relative;
}

.composer-layout {
  display: grid;
  width: 100%;
  height: 100%;
  min-height: 400px;
}

.composer-palette,
.composer-canvas {
  position: relative;
  min-width: 0;
  min-height: 0;
  width: 100%;
  height: 100%;
}

.composer-palette-resize {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 10px;
  margin-left: -5px;
  cursor: col-resize;
  z-index: 6;
}

.flow-toolbar {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  z-index: 5;
  display: flex;
  gap: 0.5rem;
}

.composer-palette-overlay {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: min(360px, 88vw);
  z-index: 20;
  display: flex;
  flex-direction: column;
  border-right: 1px solid color-mix(in srgb, var(--border-default) 80%, transparent);
  background: color-mix(in srgb, var(--bg-main) 94%, transparent);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  box-shadow: 0.75rem 0 1.75rem color-mix(in srgb, var(--text-primary) 12%, transparent);
  overflow: hidden;
}

.composer-palette-overlay-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
}

.composer-palette-overlay :deep(.catalog-shell) {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.composer-palette-backdrop {
  position: absolute;
  inset: 0;
  z-index: 18;
  border: 0;
  background: color-mix(in srgb, var(--bg-main) 40%, transparent);
  cursor: pointer;
}

.palette-overlay-enter-active,
.palette-overlay-leave-active {
  transition: transform 0.2s ease-out, visibility 0s linear 0.2s;
}

.palette-overlay-enter-to,
.palette-overlay-leave-from {
  visibility: visible;
  transform: translateX(0);
}

.palette-overlay-enter-from,
.palette-overlay-leave-to {
  visibility: hidden;
  transform: translateX(-100%);
}

.pipeline-flow {
  width: 100%;
  height: 100%;
}

.pipeline-composer :deep(.vue-flow__controls) {
  left: 0.75rem;
  right: auto;
  bottom: 0.75rem;
}

.pipeline-composer :deep(.vue-flow__minimap) {
  left: auto;
  right: 0.75rem;
  bottom: 0.75rem;
}

.edge-indicator {
  position: absolute;
  display: grid;
  place-items: center;
  width: 1.4rem;
  height: 1.4rem;
  border: 1px solid var(--border-default);
  border-radius: 999px;
  background: var(--bg-main);
  color: var(--text-primary);
  font-size: 0.78rem;
  font-weight: 700;
  box-shadow: 0 1px 4px color-mix(in srgb, var(--bg-main) 15%, transparent);
  pointer-events: none;
}

.edge-indicator--mapped {
  background: color-mix(in srgb, var(--bg-main) 82%, var(--color-status-active) 18%);
}

.edge-indicator--warning {
  border-color: var(--color-status-stuck);
  background: color-mix(in srgb, var(--bg-main) 82%, var(--color-status-stuck) 18%);
  color: color-mix(in srgb, var(--color-status-stuck) 72%, var(--text-primary));
}

.composer-empty-state {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 0.25rem;
  padding: 1rem;
  color: var(--text-muted);
  font-size: var(--text-sm);
  text-align: center;
  pointer-events: none;
}

.composer-empty-state-arrow {
  font-size: 1.5rem;
  line-height: 1;
  color: var(--text-muted);
}
</style>
