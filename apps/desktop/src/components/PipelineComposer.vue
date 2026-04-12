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
  (e: 'edgeSelected', value: { source: string; target: string } | null): void;
  (e: 'restartNode', nodeId: string): void;
  (e: 'rewindNode', nodeId: string): void;
  (e: 'forkNode', nodeId: string): void;
  (e: 'abandonNode', nodeId: string): void;
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
const { isNarrowViewport } = usePipelineMobile(computed(() => hasSelection.value));

const designGridStyle = computed(() => ({
  'grid-template-columns': !paletteOpen.value
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

watch(isNarrowViewport, (narrow) => {
  if (narrow) {
    paletteOpen.value = false;
  }
});

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
  store.addNode(
    createNode(type, { id: createNodeId(type), ...fields } as PipelineNodeInputByType[T]),
  );
  void nextTick(() => fitView());
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

const firstAgent = computed(() => configStore.agentNames[0] ?? '');
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
    closeContextMenu();
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
  editNode(contextMenu.value.nodeId);
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

function emitServerNodeAction(eventName: 'restartNode' | 'rewindNode' | 'forkNode' | 'abandonNode'): void {
  if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
  const { nodeId } = contextMenu.value;
  switch (eventName) {
    case 'restartNode':
      emit('restartNode', nodeId);
      break;
    case 'rewindNode':
      emit('rewindNode', nodeId);
      break;
    case 'forkNode':
      emit('forkNode', nodeId);
      break;
    case 'abandonNode':
      emit('abandonNode', nodeId);
      break;
  }
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
      <section v-if="paletteOpen" class="composer-palette">
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
        </div>
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
          @restart="emitServerNodeAction('restartNode')"
          @rewind="emitServerNodeAction('rewindNode')"
          @fork="emitServerNodeAction('forkNode')"
          @abandon="emitServerNodeAction('abandonNode')"
        />
      </section>
    </div>

    <template v-else>
      <div class="flow-toolbar">
        <button type="button" class="btn btn-ghost" @click="handleAutoLayout('LR')">↔ Layout LR</button>
        <button type="button" class="btn btn-ghost" @click="handleAutoLayout('TB')">↕ Layout TB</button>
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
          :elements-selectable="designMode"
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
          @close="closeContextMenu"
          @restart="emitServerNodeAction('restartNode')"
          @rewind="emitServerNodeAction('rewindNode')"
          @fork="emitServerNodeAction('forkNode')"
          @abandon="emitServerNodeAction('abandonNode')"
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

@media (max-width: 900px) {
  .composer-palette-toggle {
    display: none;
  }
}

.pipeline-flow {
  width: 100%;
  height: 100%;
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
