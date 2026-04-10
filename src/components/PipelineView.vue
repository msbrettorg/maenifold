<script setup lang="ts">
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import GraftNodesDialog from './GraftNodesDialog.vue';
import JoinPipelineDialog from './JoinPipelineDialog.vue';
import PipelineComposer from './PipelineComposer.vue';
import PipelineInspector from './PipelineInspector.vue';
import PipelineRunner from './PipelineRunner.vue';
import PipelineSidebar from './PipelineSidebar.vue';
import SemanticZoomOverlay from './SemanticZoomOverlay.vue';
import { usePipelineStore } from '../stores/pipeline';
import { usePipelineDialogs } from '../composables/usePipelineDialogs';
import { usePipelineLoading } from '../composables/usePipelineLoading';
import { usePipelineMobile } from '../composables/usePipelineMobile';
import { useResizeHandle } from '../composables/useResizeHandle';
import { getSidebarSizingTokens } from '../lib/sidebarSizing';
import type { PipelineDefinition } from '../lib/pipeline-types';

type SelectedEdge = { source: string; target: string } | null;

const store = usePipelineStore();
const { currentView, designMode, error, localDefinition, workflows } = storeToRefs(store);

const selectedNodeId = ref<string | null>(null);
const selectedEdge = ref<SelectedEdge>(null);
const runnerExpanded = ref(false);
const hasInspectorSelection = computed(() => Boolean(selectedNodeId.value || selectedEdge.value));

const { connectionError, showLoadingState, showErrorState, handleRetryLoad } = usePipelineLoading();
const { isNarrowViewport, mobileSidebarOpen, mobileInspectorOpen, toggleMobileSidebar, toggleMobileInspector } = usePipelineMobile(hasInspectorSelection);
const { graftDialogOpen, graftSubmitting, joinDialogOpen, joinSubmitting, handleGraftSubmit, handleJoinSubmit } = usePipelineDialogs();

const sidebarSizing = getSidebarSizingTokens();
const sidebarResize = useResizeHandle({
  initial: sidebarSizing.width,
  min: sidebarSizing.minWidth,
  max: sidebarSizing.maxWidth,
});
const inspectorResize = useResizeHandle({ initial: 280, min: 180, max: 500 });

const gridStyle = computed(() =>
  isNarrowViewport.value
    ? {}
    : { 'grid-template-columns': `${sidebarResize.width.value}px minmax(0, 1fr) ${inspectorResize.width.value}px` }
);

const bannerError = computed(() => connectionError.value || error.value || '');
const hasLoadedPipeline = computed(() => Boolean(currentView.value?.pipelineId));
const existingNodeIds = computed(() => currentView.value?.nodes.map((node) => node.nodeId) ?? []);

function handleDefinitionUpdate(nextDefinition: PipelineDefinition) {
  localDefinition.value = nextDefinition;
}

function handleNodeSelected(nodeId: string | null) {
  selectedNodeId.value = nodeId;
  if (nodeId) selectedEdge.value = null;
}

function handleEdgeSelected(edge: SelectedEdge) {
  selectedEdge.value = edge;
  if (edge) selectedNodeId.value = null;
}
</script>

<template>
  <div class="pipeline-view">
    <div v-if="bannerError" class="error-banner" role="alert">
      <span class="error-icon">⚠</span>
      <span class="error-text">{{ bannerError }}</span>
    </div>

    <div
      class="pipeline-grid"
      :class="{
        'runner-expanded': runnerExpanded,
        'mobile-sidebar-open': mobileSidebarOpen,
        'mobile-inspector-open': mobileInspectorOpen,
      }"
      :style="gridStyle"
    >
      <section class="sidebar-panel">
        <div v-if="showLoadingState" class="pipeline-state pipeline-state--loading" aria-live="polite">
          <div class="skeleton-block skeleton-block--title" />
          <div class="skeleton-block skeleton-block--wide" />
          <div class="skeleton-block" />
        </div>
        <div v-else-if="showErrorState" class="pipeline-state pipeline-state--error" role="alert">
          <p class="pipeline-state-title">Failed to load</p>
          <p class="pipeline-state-message">{{ connectionError }}</p>
          <button type="button" class="btn btn-primary" @click="handleRetryLoad">Retry</button>
        </div>
        <PipelineSidebar v-else />
      </section>

      <section class="composer-panel">
        <div v-if="isNarrowViewport" class="mobile-panel-controls">
          <button type="button" class="mobile-panel-toggle" @click="toggleMobileSidebar">
            {{ mobileSidebarOpen ? 'Close Sidebar' : 'Open Sidebar' }}
          </button>
          <button
            type="button"
            class="mobile-panel-toggle"
            :disabled="!hasInspectorSelection"
            @click="toggleMobileInspector"
          >
            {{ mobileInspectorOpen ? 'Close Inspector' : 'Open Inspector' }}
          </button>
        </div>
        <PipelineComposer
          :design-mode="designMode"
          :local-definition="localDefinition"
          :view="currentView"
          @update:local-definition="handleDefinitionUpdate"
          @node-selected="handleNodeSelected"
          @edge-selected="handleEdgeSelected"
        />
      </section>

      <section class="inspector-panel">
        <div v-if="isNarrowViewport" class="mobile-inspector-header">
          <span>Inspector</span>
          <button type="button" class="mobile-inspector-close" @click="mobileInspectorOpen = false">
            Close
          </button>
        </div>
        <PipelineInspector
          :selected-node-id="selectedNodeId"
          :selected-edge="selectedEdge"
        />
      </section>

      <section class="runner-panel">
        <div class="runner-panel-header">
          <button
            v-if="hasLoadedPipeline"
            type="button"
            class="btn btn-primary"
            :disabled="graftSubmitting"
            @click="graftDialogOpen = true"
          >
            Graft Nodes
          </button>
          <button
            v-if="hasLoadedPipeline"
            type="button"
            class="btn btn-ghost"
            :disabled="joinSubmitting"
            @click="joinDialogOpen = true"
          >
            Join Pipeline
          </button>
          <button
            class="runner-expand-btn"
            type="button"
            aria-label="Toggle runner panel"
            :title="runnerExpanded ? 'Collapse panel' : 'Expand panel'"
            @click="runnerExpanded = !runnerExpanded"
          >
            {{ runnerExpanded ? '▼' : '▲' }}
          </button>
        </div>
        <PipelineRunner />
      </section>

      <!-- Resize handles (desktop only) -->
      <div
        v-if="!isNarrowViewport"
        class="grid-resize-handle grid-resize-handle--sidebar"
        :style="{ left: sidebarResize.width.value + 'px' }"
        @mousedown="sidebarResize.startResize($event, 'right')"
      />
      <div
        v-if="!isNarrowViewport"
        class="grid-resize-handle grid-resize-handle--inspector"
        :style="{ right: inspectorResize.width.value + 'px' }"
        @mousedown="inspectorResize.startResize($event, 'left')"
      />

      <button
        v-if="isNarrowViewport && mobileSidebarOpen"
        type="button"
        class="mobile-overlay"
        aria-label="Close sidebar"
        @click="mobileSidebarOpen = false"
      />
      <button
        v-if="isNarrowViewport && mobileInspectorOpen"
        type="button"
        class="mobile-overlay"
        aria-label="Close inspector"
        @click="mobileInspectorOpen = false"
      />
    </div>

    <GraftNodesDialog
      v-if="graftDialogOpen && currentView"
      :busy="graftSubmitting"
      :existing-node-ids="existingNodeIds"
      :workflows="workflows"
      @cancel="graftDialogOpen = false"
      @submit="handleGraftSubmit"
    />
    <JoinPipelineDialog
      v-if="joinDialogOpen && currentView"
      :busy="joinSubmitting"
      @cancel="joinDialogOpen = false"
      @submit="handleJoinSubmit"
    />
    <SemanticZoomOverlay />
  </div>
</template>

<style scoped>
@import './PipelineView.css';
</style>
