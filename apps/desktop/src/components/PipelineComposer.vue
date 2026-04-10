<script setup lang="ts">
import { nextTick } from 'vue';
import { BaseEdge, EdgeLabelRenderer, VueFlow } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Controls } from '@vue-flow/controls';
import { MiniMap } from '@vue-flow/minimap';

import '@vue-flow/core/dist/style.css';
import '@vue-flow/core/dist/theme-default.css';
import '@vue-flow/controls/dist/style.css';
import '@vue-flow/minimap/dist/style.css';

import AddNodeForm from './AddNodeForm.vue';
import { useComposerCanvas } from '../composables/useComposerCanvas';
import type {
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
}>();

const {
  defaultEdgeOptions,
  deleteSelected,
  edgeIndicator,
  edgePath,
  edgeTooltip,
  edges,
  fitView,
  handleAutoLayout,
  handleFlowKeyDown,
  hasSelection,
  nodeTypes,
  nodes,
  showEmptyState,
} = useComposerCanvas({
  designMode: () => props.designMode,
  localDefinition: () => props.localDefinition,
  view: () => props.view,
  updateDefinition: (definition) => emit('update:localDefinition', definition),
  selectNode: (nodeId) => emit('nodeSelected', nodeId),
  selectEdge: (edge) => emit('edgeSelected', edge),
});

const store = usePipelineStore();

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

if (props.designMode && store.workflows.length === 0) {
  void store.refreshWorkflows();
}

defineExpose({ deleteSelected, fitView, handleFlowKeyDown });
</script>

<template>
  <div class="pipeline-composer" @keydown="handleFlowKeyDown">
    <AddNodeForm
      v-if="designMode"
      :workflows="store.workflows"
      :has-selection="hasSelection"
      :submit-node="submitNode"
      @auto-layout="handleAutoLayout"
      @delete-selected="deleteSelected"
    />
    <div v-else class="flow-toolbar">
      <button type="button" class="btn btn-ghost" @click="handleAutoLayout('LR')">↔ Layout LR</button>
      <button type="button" class="btn btn-ghost" @click="handleAutoLayout('TB')">↕ Layout TB</button>
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
      <span>Add your first node</span>
      <span class="composer-empty-state-arrow" aria-hidden="true">↖</span>
    </div>
  </div>
</template>

<style scoped>
.pipeline-composer {
  width: 100%;
  height: 100%;
  min-height: 400px;
  position: relative;
}

.flow-toolbar {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  z-index: 5;
  display: flex;
  gap: 0.5rem;
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
  border: 1px solid var(--border-color);
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
