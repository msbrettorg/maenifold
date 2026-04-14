<script setup lang="ts">
import { computed, nextTick } from 'vue';
import { BaseEdge, EdgeLabelRenderer, VueFlow } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Controls } from '@vue-flow/controls';
import { MiniMap } from '@vue-flow/minimap';
import { ContextMenu, ContextMenuTrigger } from '@/components/ui/context-menu';

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
  createNodeAt, defaultEdgeOptions, deleteSelected, deleteNode, disconnectNodeEdges, duplicateNode, edgeIndicator, edgePath,
  edgeTooltip, editNode, edges, fitView, handleAutoLayout, handleFlowKeyDown, hasSelection, nodeTypes, nodes, showEmptyState, toFlowPosition,
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

const NODE_TYPE_COLORS: Record<string, string> = { workflow: '#4d8eff', prompt: '#adc6ff', sequentialThinking: '#d0bcff', tool: '#4edea3' };
const miniMapNodeColor = (node: { data?: { type?: string } }) => NODE_TYPE_COLORS[node.data?.type ?? ''] ?? NODE_TYPE_COLORS.workflow;
const contextMenuEnabled = computed(() => contextMenu.value !== null);

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

            <Transition name="palette-overlay">
              <section v-if="paletteAsOverlay" class="composer-palette-overlay">
                <div class="composer-palette-overlay-header">
                  <strong>Add Node</strong>
                  <button type="button" class="btn btn-ghost btn-xs" @click="closePalette">✕ Close</button>
                </div>
                <AddNodeForm :workflows="store.workflows" :submit-node="submitNode" />
              </section>
            </Transition>
            <button v-if="paletteAsOverlay" type="button" class="composer-palette-backdrop" aria-label="Close palette" @click="closePalette" />

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
              <Background color="#2a2a2a" />
              <Controls :show-interactive="designMode" />
              <MiniMap :node-color="miniMapNodeColor" mask-color="rgba(0,0,0,0.25)" />
              <template #edge-smoothstep="edgeProps">
                <g>
                  <BaseEdge :id="edgeProps.id" :path="edgePath(edgeProps).d" :style="edgeProps.style" :marker-start="edgeProps.markerStart" :marker-end="edgeProps.markerEnd" :interaction-width="edgeProps.interactionWidth" />
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
.composer-palette-overlay { position: absolute; top: 0; left: 0; bottom: 0; width: min(360px, 88vw); z-index: 20; display: flex; flex-direction: column; border-right: 1px solid color-mix(in srgb, var(--border) 80%, transparent); background: color-mix(in srgb, var(--background) 94%, transparent); backdrop-filter: blur(14px); -webkit-backdrop-filter: blur(14px); box-shadow: 0.75rem 0 1.75rem color-mix(in srgb, var(--foreground) 12%, transparent); overflow: hidden; }
.composer-palette-overlay-header { display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--border); flex-shrink: 0; }
.composer-palette-overlay :deep(.catalog-shell) { flex: 1; min-height: 0; overflow-y: auto; }
.composer-palette-backdrop { position: absolute; inset: 0; z-index: 18; border: 0; background: color-mix(in srgb, var(--background) 40%, transparent); cursor: pointer; }
.palette-overlay-enter-active, .palette-overlay-leave-active { transition: transform 0.2s ease-out, visibility 0s linear 0.2s; }
.palette-overlay-enter-to, .palette-overlay-leave-from { visibility: visible; transform: translateX(0); }
.palette-overlay-enter-from, .palette-overlay-leave-to { visibility: hidden; transform: translateX(-100%); }
.pipeline-composer :deep(.vue-flow__controls) { left: 0.75rem; right: auto; bottom: 0.75rem; }
.pipeline-composer :deep(.vue-flow__minimap) { left: auto; right: 0.75rem; bottom: 0.75rem; }
.edge-indicator { position: absolute; display: grid; place-items: center; width: 1.4rem; height: 1.4rem; border: 1px solid var(--border); border-radius: 999px; background: var(--background); color: var(--foreground); font-size: var(--text-sm); font-weight: var(--weight-title); box-shadow: 0 1px 4px color-mix(in srgb, var(--background) 15%, transparent); pointer-events: none; }
.edge-indicator--mapped { background: color-mix(in srgb, var(--background) 82%, var(--color-status-active) 18%); }
.edge-indicator--warning { border-color: var(--color-status-stuck); background: color-mix(in srgb, var(--background) 82%, var(--color-status-stuck) 18%); color: color-mix(in srgb, var(--color-status-stuck) 72%, var(--foreground)); }
.composer-empty-state { position: absolute; inset: 0; display: grid; place-items: center; align-content: center; gap: 0.25rem; padding: 1rem; color: var(--muted-foreground); font-size: var(--text-sm); text-align: center; pointer-events: none; }
.composer-empty-state-arrow { font-size: var(--text-2xl); line-height: 1; color: var(--muted-foreground); }
</style>
