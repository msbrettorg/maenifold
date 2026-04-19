<script setup lang="ts">
import { computed, markRaw, nextTick, provide, ref, watch } from 'vue';
import { VueFlow } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Controls } from '@vue-flow/controls';
import { MiniMap } from '@vue-flow/minimap';
import { Button } from '@/components/ui/button';
import { ContextMenu, ContextMenuTrigger } from '@/components/ui/context-menu';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet';

import '@vue-flow/core/dist/style.css';
import '@vue-flow/core/dist/theme-default.css';
import '@vue-flow/controls/dist/style.css';
import '@vue-flow/minimap/dist/style.css';

import AddNodeForm from './AddNodeForm.vue';
import PipelineComposerContextMenu from './PipelineComposerContextMenu.vue';
import PipelineFlowEdge from './PipelineFlowEdge.vue';
import PipelineStatusBadge from './PipelineStatusBadge.vue';
import PipelineComposerToolbar from './PipelineComposerToolbar.vue';
import { useComposerCanvas } from '../composables/useComposerCanvas';
import { useComposerPalette } from '../composables/useComposerPalette';
import { useComposerContextMenu } from '../composables/useComposerContextMenu';
import { useMcpToolDiscovery } from '../composables/useMcpToolDiscovery';
import type { PipelineDefinition, PipelineNodeType, PipelineView } from '../lib/pipeline-types';
import { createNode, createNodeId, type PipelineNodeDraftByType, type PipelineNodeInputByType } from '../lib/pipeline-node-factory';
import { NODE_TYPE_COLORS } from '../lib/pipeline-status-tokens';
import { getBareToolName } from '../lib/tool-categories';
import { resolvePreferredAgent } from '../lib/preferred-agent';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{ designMode: boolean; localDefinition?: PipelineDefinition; view?: PipelineView | null }>();
const emit = defineEmits<{
  (e: 'update:localDefinition', value: PipelineDefinition): void;
  (e: 'nodeSelected', nodeId: string | null): void;
  (e: 'nodeDoubleClick', nodeId: string): void;
  (e: 'edgeDoubleClick', value: { source: string; target: string }): void;
  (e: 'edgeSelected', value: { source: string; target: string } | null): void;
  (e: 'openGraft'): void;
  (e: 'openPromotion'): void;
}>();

const store = usePipelineStore();
const configStore = useConfigStore();
const layoutDirection = ref<'LR' | 'TB'>('LR');
provide('pipelineLayoutDirection', layoutDirection);
const {
  createNodeAt, defaultEdgeOptions, deleteSelected, deleteNode, disconnectNodeEdges, duplicateNode, editNode, edges, fitView,
  handleAutoLayout, handleFlowKeyDown, hasSelection, nodeTypes, nodes, showEmptyState, toFlowPosition,
} = useComposerCanvas({
  designMode: () => props.designMode,
  localDefinition: () => props.localDefinition,
  view: () => props.view,
  layoutDirection,
  updateDefinition: (definition) => emit('update:localDefinition', definition),
  selectNode: (nodeId) => emit('nodeSelected', nodeId),
  selectEdge: (edge) => emit('edgeSelected', edge),
  designNodeDoubleClick: (nodeId) => emit('nodeDoubleClick', nodeId),
  edgeDoubleClick: (edge) => emit('edgeDoubleClick', edge),
});

const { composerLayoutStyle, closePalette, paletteAsOverlay, paletteOpen, paletteResize, showPalettePanel, togglePalette } =
  useComposerPalette(() => props.designMode);

const { serverResults, hasLoadedCatalog, refreshTools } = useMcpToolDiscovery();

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

const currentPipelineStatus = computed(() => props.view?.status ?? null);
const currentPipelineRunning = computed(() => props.view?.running ?? false);
const hasRewindableNodes = computed(() =>
  props.view?.nodes.some((node) => ['completed', 'abandoned', 'stuck'].includes(node.status)) ?? false,
);
const hasCompletedNodesWithOutput = computed(() =>
  props.view?.nodes.some((node) => node.status === 'completed' && Boolean(node.output)) ?? false,
);

const {
  contextMenu, contextMenuEnabled, contextMenuNodeStatus, contextMenuNodeHasOutput,
  closeContextMenu, handleComposerContextMenu, handleEditNode, handleDuplicateNode,
  handleDeleteNode, handleDisconnectEdges, handleServerNodeAction,
  handleCanvasRunPipeline, handleCanvasRewindAll, handleCanvasGraft, handleCanvasPromoteToMemory,
  handleContextMenuOpenChange, createPromptNode, createThinkingNode, createToolNode, createWorkflowNode,
} = useComposerContextMenu({
  designMode: () => props.designMode,
  view: () => props.view,
  firstAgent,
  firstWorkflow,
  createNodeAt,
  editNode,
  duplicateNode,
  deleteNode,
  disconnectNodeEdges,
  toFlowPosition,
  emitNodeDoubleClick: (nodeId) => emit('nodeDoubleClick', nodeId),
  emitOpenGraft: () => emit('openGraft'),
  emitOpenPromotion: () => emit('openPromotion'),
});

function maybeRefreshComposerCatalogs(): void {
  if (!props.designMode) return;
  if (store.workflows.length === 0) void store.refreshWorkflows();
  if (!hasLoadedCatalog.value) void refreshTools();
}

const edgeTypes = { smoothstep: markRaw(PipelineFlowEdge) };
watch(() => props.designMode, maybeRefreshComposerCatalogs, { immediate: true });

const miniMapNodeColor = (node: { data?: { type?: string } }) => NODE_TYPE_COLORS[node.data?.type ?? ''] ?? NODE_TYPE_COLORS.workflow;

function submitNode<T extends PipelineNodeType>(type: T, fields: PipelineNodeDraftByType[T]): void {
  const node = store.addNode(createNode(type, { id: createNodeId(type), ...fields } as PipelineNodeInputByType[T]));
  void nextTick(() => { editNode(node.id); fitView(); });
}

function handleLayoutDirectionChange(direction: 'LR' | 'TB'): void {
  layoutDirection.value = direction;
}
</script>

<template>
  <div class="pipeline-composer" role="application" aria-label="Pipeline canvas" @keydown="handleFlowKeyDown">
    <div class="composer-layout" :style="composerLayoutStyle">
      <section v-if="showPalettePanel" class="composer-palette">
        <AddNodeForm :workflows="store.workflows" :submit-node="submitNode" />
        <div class="composer-palette-resize" :style="{ left: `${paletteResize.width.value}px` }" @mousedown="paletteResize.startResize($event, 'right')" />
      </section>

      <ContextMenu @update:open="handleContextMenuOpenChange">
        <ContextMenuTrigger as-child :disabled="!contextMenuEnabled">
          <section class="composer-canvas" @contextmenu.capture="handleComposerContextMenu">
            <PipelineComposerToolbar
              :design-mode="designMode"
              :has-selection="hasSelection"
              :layout-direction="layoutDirection"
              :palette-open="paletteOpen"
              :show-palette-toggle="designMode"
              @auto-layout="handleAutoLayout"
              @set-layout-direction="handleLayoutDirectionChange"
              @delete-selected="deleteSelected"
              @fit-view="fitView"
              @toggle-palette="togglePalette"
            >
              <slot name="toolbar-actions" />
            </PipelineComposerToolbar>
            <PipelineStatusBadge :status="view?.status ?? null" />

              <Sheet :open="paletteAsOverlay" @update:open="(open) => !open && closePalette()">
                <SheetContent
                  v-if="paletteAsOverlay"
                 side="left"
                 :show-close-button="false"
                  class="composer-palette-sheet w-[min(360px,88vw)] border-0 p-0 sm:max-w-[360px]"
               >
                  <SheetHeader class="flex flex-row items-center justify-between gap-3 border-0 px-3 py-3">
                    <SheetTitle class="font-display text-[var(--text-title)] font-medium text-[var(--text)]">Add node</SheetTitle>
                   <Button type="button" variant="ghost" size="sm" class="h-8 rounded-[4px] px-2 text-xs" @click="closePalette">Close</Button>
                 </SheetHeader>
                 <ScrollArea class="h-[calc(100vh-4.5rem)]">
                   <div class="composer-palette-sheet-body">
                     <AddNodeForm :workflows="store.workflows" :submit-node="submitNode" />
                   </div>
                 </ScrollArea>
               </SheetContent>
             </Sheet>

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
               :edge-types="edgeTypes"
               :nodes-draggable="designMode"
               :nodes-connectable="designMode"
              :edges-updatable="designMode"
              :elements-selectable="true"
              :delete-key-code="designMode ? 'Backspace' : null"
              :fit-view-on-init="designMode"
              class="pipeline-flow"
            >
               <Background :color="'var(--surface-high)'" />
              <Controls :show-interactive="designMode" />
               <MiniMap :node-color="miniMapNodeColor" :mask-color="'color-mix(in srgb, var(--color-bg-app) 75%, transparent)'" />
            </VueFlow>

            <div v-if="showEmptyState" class="composer-empty-state">
              <span>{{ designMode ? 'Add nodes using the form or right-click the canvas' : 'Add your first node' }}</span>
              <span v-if="!designMode" class="composer-empty-state-arrow" aria-hidden="true">↖</span>
            </div>
          </section>
        </ContextMenuTrigger>

        <PipelineComposerContextMenu
          v-if="contextMenu"
          :mode="designMode ? 'design' : 'server'"
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
          @run-pipeline="handleCanvasRunPipeline"
          @rewind-all="handleCanvasRewindAll"
          @graft="handleCanvasGraft"
          @promote-to-memory="handleCanvasPromoteToMemory"
        />
      </ContextMenu>
    </div>
  </div>
</template>

<style scoped>
.pipeline-composer, .composer-layout, .composer-palette, .composer-canvas, .pipeline-flow { width: 100%; height: 100%; min-height: 0; }
.pipeline-composer { min-height: 400px; position: relative; }
.composer-layout { display: grid; }
.composer-palette, .composer-canvas { position: relative; min-width: 0; }
.composer-palette-resize { position: absolute; top: 0; bottom: 0; width: 10px; margin-left: -5px; cursor: col-resize; z-index: 6; }
.composer-palette-sheet-body { min-height: 0; padding: 0 0 1rem; }
.composer-palette-sheet {
  background: var(--surface);
  color: var(--text);
  box-shadow: var(--shadow-16);
}
.pipeline-composer :deep(.vue-flow__controls) { left: 0.75rem; right: auto; bottom: 0.75rem; background: var(--surface-high); border-radius: var(--radius); }
.pipeline-composer :deep(.vue-flow__controls button) { background: transparent; color: var(--color-text-primary); border: none; }
.pipeline-composer :deep(.vue-flow__controls button:hover) { background: var(--color-bg-surface); }
.pipeline-composer :deep(.vue-flow__minimap) { left: auto; right: 0.75rem; bottom: 0.75rem; background: var(--surface-high); border-radius: var(--radius); }
.pipeline-composer :deep(.vue-flow__minimap svg) { background: transparent; }
.composer-empty-state { position: absolute; inset: 0; display: grid; place-items: center; align-content: center; gap: 0.25rem; padding: 1rem; color: var(--color-text-secondary); font-size: var(--text-sm); text-align: center; pointer-events: none; }
.composer-empty-state-arrow { font-size: var(--text-2xl); line-height: 1; color: var(--color-text-secondary); }
</style>
