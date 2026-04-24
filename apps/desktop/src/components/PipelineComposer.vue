<script setup lang="ts">
import { computed, markRaw, nextTick, provide, ref, watch } from 'vue';
import { VueFlow } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Button } from '@/components/ui/button';
import { ContextMenu, ContextMenuTrigger } from '@/components/ui/context-menu';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet';

import '@vue-flow/core/dist/style.css';
import '@vue-flow/core/dist/theme-default.css';

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


function submitNode<T extends PipelineNodeType>(type: T, fields: PipelineNodeDraftByType[T]): void {
  const node = store.addNode(createNode(type, { id: createNodeId(type), ...fields } as PipelineNodeInputByType[T]));
  void nextTick(() => { editNode(node.id); fitView(); });
}

function handleLayoutDirectionChange(direction: 'LR' | 'TB'): void {
  layoutDirection.value = direction;
}
</script>

<template>
  <div class="relative w-full h-full min-h-[400px]" role="application" aria-label="Pipeline canvas" @keydown="handleFlowKeyDown">
    <div class="grid w-full h-full min-h-0" :style="composerLayoutStyle">
      <section v-if="showPalettePanel" class="relative w-full h-full min-w-0 min-h-0">
        <AddNodeForm :workflows="store.workflows" :submit-node="submitNode" />
        <div
          class="absolute top-0 bottom-0 w-[10px] -ml-[5px] cursor-col-resize z-[6]"
          :style="{ left: `${paletteResize.width.value}px` }"
          @mousedown="paletteResize.startResize($event, 'right')"
        />
      </section>

      <ContextMenu @update:open="handleContextMenuOpenChange">
        <ContextMenuTrigger as-child :disabled="!contextMenuEnabled">
          <section class="relative w-full h-full min-w-0 min-h-0" @contextmenu.capture="handleComposerContextMenu">
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
                class="w-[min(360px,88vw)] border-0 p-0 sm:max-w-[360px] bg-surface text-text-primary shadow-fluent-16"
              >
                <SheetHeader class="flex flex-row items-center justify-between gap-3 border-0 px-3 py-3">
                  <SheetTitle class="font-display font-medium text-text-primary text-title">Add node</SheetTitle>
                  <Button type="button" variant="ghost" size="sm" class="h-8 rounded-[4px] px-2 text-xs" @click="closePalette">Close</Button>
                </SheetHeader>
                <ScrollArea class="h-[calc(100vh-4.5rem)]">
                  <div class="min-h-0 pb-4">
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
              class="w-full h-full min-h-0"
            >
              <Background class="text-surface-high" />
            </VueFlow>

            <div
              v-if="showEmptyState"
              class="absolute inset-0 grid place-items-center content-center gap-1 p-4 text-text-secondary text-sm text-center pointer-events-none"
            >
              <span>{{ designMode ? 'Add nodes using the form or right-click the canvas' : 'Add your first node' }}</span>
              <span v-if="!designMode" class="text-2xl leading-none text-text-secondary" aria-hidden="true">↖</span>
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
