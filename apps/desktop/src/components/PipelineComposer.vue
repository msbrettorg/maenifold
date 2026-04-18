<script setup lang="ts">
import { computed, nextTick } from 'vue';
import { BaseEdge, EdgeLabelRenderer, VueFlow } from '@vue-flow/core';
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
import PipelineComposerToolbar from './PipelineComposerToolbar.vue';
import { useComposerCanvas } from '../composables/useComposerCanvas';
import { useComposerContextMenu } from '../composables/useComposerContextMenu';
import { useComposerPalette } from '../composables/useComposerPalette';
import type { PipelineDefinition, PipelineNodeType, PipelineView } from '../lib/pipeline-types';
import { createNode, createNodeId, type PipelineNodeDraftByType, type PipelineNodeInputByType } from '../lib/pipeline-node-factory';
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
const {
  createNodeAt, defaultEdgeOptions, deleteSelected, deleteNode, disconnectNodeEdges, duplicateNode, edgeIndicator, edgeLabel, edgePath,
  edgeLabelTitle, edgePathStyle, edgeTooltip, editNode, edges, fitView, handleAutoLayout, handleFlowKeyDown, hasSelection, nodeTypes, nodes, showEmptyState, toFlowPosition,
} = useComposerCanvas({
  designMode: () => props.designMode,
  localDefinition: () => props.localDefinition,
  view: () => props.view,
  updateDefinition: (definition) => emit('update:localDefinition', definition),
  selectNode: (nodeId) => emit('nodeSelected', nodeId),
  selectEdge: (edge) => emit('edgeSelected', edge),
  designNodeDoubleClick: (nodeId) => emit('nodeDoubleClick', nodeId),
  edgeDoubleClick: (edge) => emit('edgeDoubleClick', edge),
});

const { composerLayoutStyle, closePalette, paletteAsOverlay, paletteOpen, paletteResize, showPalettePanel, togglePalette } =
  useComposerPalette(() => props.designMode);
const {
  closeContextMenu,
  contextMenu,
  contextMenuNodeHasOutput,
  contextMenuNodeStatus,
  createPromptNode,
  createThinkingNode,
  createToolNode,
  createWorkflowNode,
  currentPipelineRunning,
  currentPipelineStatus,
  firstAgent,
  firstWorkflow,
  handleCanvasGraft,
  handleCanvasPromoteToMemory,
  handleCanvasRewindAll,
  handleCanvasRunPipeline,
  handleComposerContextMenu,
  handleDeleteNode,
  handleDisconnectEdges,
  handleDuplicateNode,
  handleEditNode,
  handleServerNodeAction,
  hasCompletedNodesWithOutput,
  hasRewindableNodes,
  toolGroups,
} = useComposerContextMenu({
  designMode: () => props.designMode,
  view: () => props.view,
  toFlowPosition,
  createNodeAtPosition: createNodeAt,
  editNode,
  duplicateNode,
  deleteNode,
  disconnectNodeEdges,
  emitNodeDoubleClick: (nodeId) => emit('nodeDoubleClick', nodeId),
  emitOpenGraft: () => emit('openGraft'),
  emitOpenPromotion: () => emit('openPromotion'),
});

const NODE_TYPE_COLORS: Record<string, string> = {
  workflow: 'var(--blue)',
  prompt: 'color-mix(in srgb, var(--blue) 55%, var(--text) 45%)',
  sequentialThinking: 'var(--plum)',
  tool: 'var(--sage)',
};
const miniMapNodeColor = (node: { data?: { type?: string } }) => NODE_TYPE_COLORS[node.data?.type ?? ''] ?? NODE_TYPE_COLORS.workflow;
const contextMenuEnabled = computed(() => contextMenu.value !== null);
const STATUS_COLORS: Record<string, string> = {
  active: 'var(--blue)',
  completed: 'var(--sage)',
  stuck: 'var(--terracotta)',
  abandoned: 'var(--text-dim)',
};
const pipelineStatusBadge = computed(() => {
  const status = props.view?.status;
  if (!status) {
    return null;
  }
  const color = STATUS_COLORS[status] ?? 'var(--text-dim)';
  return { label: status, color };
});

function submitNode<T extends PipelineNodeType>(type: T, fields: PipelineNodeDraftByType[T]): void {
  const node = store.addNode(createNode(type, { id: createNodeId(type), ...fields } as PipelineNodeInputByType[T]));
  void nextTick(() => {
    editNode(node.id);
    fitView();
  });
}

function handleContextMenuOpenChange(open: boolean): void {
  if (!open) closeContextMenu();
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
              :palette-open="paletteOpen"
              :show-palette-toggle="designMode"
              @auto-layout="handleAutoLayout"
              @delete-selected="deleteSelected"
              @fit-view="fitView"
              @toggle-palette="togglePalette"
            >
              <slot name="toolbar-actions" />
            </PipelineComposerToolbar>
            <div v-if="pipelineStatusBadge" class="composer-status-badge" :aria-label="`Pipeline status: ${pipelineStatusBadge.label}`">
              <span class="composer-status-dot" :style="{ backgroundColor: pipelineStatusBadge.color }" aria-hidden="true" />
              <span>{{ pipelineStatusBadge.label }}</span>
            </div>

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
               <MiniMap :node-color="miniMapNodeColor" :mask-color="'color-mix(in srgb, var(--background) 75%, transparent)'" />
              <template #edge-smoothstep="edgeProps">
                <g>
                  <BaseEdge :id="edgeProps.id" :path="edgePath(edgeProps).d" :style="edgePathStyle(edgeProps)" :marker-start="edgeProps.markerStart" :marker-end="edgeProps.markerEnd" :interaction-width="edgeProps.interactionWidth" />
                  <title>{{ edgeTooltip(edgeProps.data) }}</title>
                </g>
                <EdgeLabelRenderer v-if="edgeLabel(edgeProps.data)">
                  <div
                    class="edge-condition-label"
                    :style="{ transform: `translate(-50%, -50%) translate(${edgePath(edgeProps).x}px, ${edgePath(edgeProps).y}px)` }"
                    :title="edgeLabelTitle(edgeProps.data) ?? undefined"
                  >
                    {{ edgeLabel(edgeProps.data) }}
                  </div>
                </EdgeLabelRenderer>
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
  box-shadow: 0 0 32px color-mix(in srgb, var(--blue) 8%, transparent);
}
.pipeline-composer :deep(.vue-flow__controls) { left: 0.75rem; right: auto; bottom: 0.75rem; background: var(--surface-high); border-radius: var(--radius); }
.pipeline-composer :deep(.vue-flow__controls button) { background: transparent; color: var(--foreground); border: none; }
.pipeline-composer :deep(.vue-flow__controls button:hover) { background: var(--muted); }
.pipeline-composer :deep(.vue-flow__minimap) { left: auto; right: 0.75rem; bottom: 0.75rem; background: var(--surface-high); border-radius: var(--radius); }
.pipeline-composer :deep(.vue-flow__minimap svg) { background: transparent; }
.composer-status-badge {
  position: absolute;
  top: 0.75rem;
  left: 0.75rem;
  z-index: 7;
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem 0.6rem;
  border-radius: 4px;
  background: var(--surface-container);
  color: var(--text);
  font-size: var(--text-xs);
  text-transform: lowercase;
}
.composer-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  flex-shrink: 0;
}
.edge-indicator { position: absolute; display: grid; place-items: center; width: 1.4rem; height: 1.4rem; border-radius: 4px; background: color-mix(in srgb, var(--background) 92%, var(--surface-high) 8%); color: var(--foreground); font-size: var(--text-sm); font-weight: var(--weight-title); box-shadow: 0 1px 4px color-mix(in srgb, var(--background) 15%, transparent); pointer-events: none; }
.edge-indicator--mapped { background: color-mix(in srgb, var(--background) 82%, var(--color-status-active) 18%); }
.edge-indicator--warning { border-color: var(--color-status-stuck); background: color-mix(in srgb, var(--background) 82%, var(--color-status-stuck) 18%); color: color-mix(in srgb, var(--color-status-stuck) 72%, var(--foreground)); }
.edge-condition-label {
  position: absolute;
  padding: 0.15rem 0.35rem;
  border-radius: 4px;
  background: var(--surface-high);
  color: var(--text);
  font-size: var(--text-xs);
  line-height: 1.2;
  pointer-events: none;
  max-width: 18ch;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.composer-empty-state { position: absolute; inset: 0; display: grid; place-items: center; align-content: center; gap: 0.25rem; padding: 1rem; color: var(--muted-foreground); font-size: var(--text-sm); text-align: center; pointer-events: none; }
.composer-empty-state-arrow { font-size: var(--text-2xl); line-height: 1; color: var(--muted-foreground); }
</style>
