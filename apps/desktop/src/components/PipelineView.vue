<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import GraftNodesDialog from './GraftNodesDialog.vue';
import JoinPipelineDialog from './JoinPipelineDialog.vue';
import PipelineComposer from './PipelineComposer.vue';
import PipelineFlyout from './PipelineFlyout.vue';
import PipelineIconRail from './PipelineIconRail.vue';
import PipelineInspector from './PipelineInspector.vue';
import PipelineRunner from './PipelineRunner.vue';
import PipelineSidebar from './PipelineSidebar.vue';
import SemanticZoomOverlay from './SemanticZoomOverlay.vue';
import TemplateGallery from './TemplateGallery.vue';
import { useFlyout } from '../composables/useFlyout';
import { usePipelineStore } from '../stores/pipeline';
import { usePipelineDialogs } from '../composables/usePipelineDialogs';
import { usePipelineLoading } from '../composables/usePipelineLoading';
import { usePipelineMobile } from '../composables/usePipelineMobile';
import type { PipelineDefinition } from '../lib/pipeline-types';

type SelectedEdge = { source: string; target: string } | null;
type PipelineIconRailExpose = { focusPipelineToggle: () => void };

const store = usePipelineStore();
const { currentPipelineId, currentView, designMode, error, localDefinition, workflows } = storeToRefs(store);
const selectedNodeId = ref<string | null>(null);
const selectedEdge = ref<SelectedEdge>(null);
const runnerExpanded = ref(false);
const iconRailRef = ref<PipelineIconRailExpose | null>(null);
const hasInspectorSelection = computed(() => Boolean(selectedNodeId.value || selectedEdge.value));
const { connectionError, showLoadingState, showErrorState, handleRetryLoad } = usePipelineLoading();
const { isNarrowViewport, mobileSidebarOpen, mobileInspectorOpen, toggleMobileSidebar, toggleMobileInspector } = usePipelineMobile(hasInspectorSelection);
const { graftDialogOpen, graftSubmitting, joinDialogOpen, joinSubmitting, handleGraftSubmit, handleJoinSubmit } = usePipelineDialogs();
const { activeFlyout, close: closeFlyout, toggle: toggleFlyout } = useFlyout();
const isInspectorVisible = computed(() => isNarrowViewport.value ? mobileInspectorOpen.value : hasInspectorSelection.value);
const isPipelineFlyoutOpen = computed(() => !isNarrowViewport.value && activeFlyout.value === 'pipelines');
const bannerError = computed(() => connectionError.value || error.value || '');
const hasLoadedPipeline = computed(() => Boolean(currentView.value?.pipelineId));
const existingNodeIds = computed(() => currentView.value?.nodes.map((node) => node.nodeId) ?? []);
const activePipelineLabel = computed(() => currentView.value?.pipelineId ?? currentPipelineId.value ?? '');
const showFullScreenEmpty = computed(() =>
  designMode.value
  && !currentView.value
  && (localDefinition.value?.nodes?.length ?? 0) === 0
);

watch(() => error.value, (message) => { if (message) runnerExpanded.value = true; }, { immediate: true });
watch(() => String(currentView.value?.status ?? ''), (status) => {
  if (status === 'stuck' || status === 'abandoned') runnerExpanded.value = true;
}, { immediate: true });
watch(() => designMode.value && (localDefinition.value?.nodes?.length ?? 0) > 0, (hasContent) => {
  if (hasContent) runnerExpanded.value = true;
}, { immediate: true });
watch(isNarrowViewport, (narrow) => { if (narrow) closeFlyout(); });

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
function clearInspectorSelection() {
  selectedNodeId.value = null;
  selectedEdge.value = null;
}
function focusRailToggle() {
  nextTick(() => iconRailRef.value?.focusPipelineToggle());
}
function closePipelineFlyout(options?: { restoreFocus?: boolean }) {
  if (!isPipelineFlyoutOpen.value) return;
  closeFlyout();
  if (options?.restoreFocus) focusRailToggle();
}
function handleTogglePipelineFlyout() {
  toggleFlyout('pipelines');
}
function handlePipelineSelected() {
  clearInspectorSelection();
  closePipelineFlyout();
}
function handleNewPipeline() {
  clearInspectorSelection();
  store.newPipeline();
  closePipelineFlyout();
  if (isNarrowViewport.value) {
    mobileSidebarOpen.value = false;
    mobileInspectorOpen.value = false;
  }
}
function handleFlyoutBackdropClick(event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  closePipelineFlyout();
}
function handleFlyoutBackdropContextMenu(event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  closePipelineFlyout();
}
function handleWindowKeyDown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  if (isPipelineFlyoutOpen.value) {
    event.preventDefault();
    closePipelineFlyout({ restoreFocus: true });
    return;
  }
  if (!isInspectorVisible.value) return;
  clearInspectorSelection();
  if (isNarrowViewport.value) mobileInspectorOpen.value = false;
}

onMounted(() => window.addEventListener('keydown', handleWindowKeyDown));
onBeforeUnmount(() => window.removeEventListener('keydown', handleWindowKeyDown));
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
        'grid-empty': showFullScreenEmpty,
        'runner-expanded': runnerExpanded,
        'mobile-sidebar-open': mobileSidebarOpen,
        'mobile-inspector-open': mobileInspectorOpen,
      }"
    >
      <section v-if="!isNarrowViewport" class="rail-panel">
        <PipelineIconRail
          ref="iconRailRef"
          :active-pipeline-label="activePipelineLabel"
          :flyout-open="isPipelineFlyoutOpen"
          :is-narrow-viewport="isNarrowViewport"
          @toggle-flyout="handleTogglePipelineFlyout"
          @new-pipeline="handleNewPipeline"
        />
      </section>
      <section v-if="isNarrowViewport" class="sidebar-panel">
        <PipelineSidebar />
      </section>
      <section class="composer-panel">
        <div v-if="isNarrowViewport" class="mobile-panel-controls">
          <button type="button" class="mobile-panel-toggle" @click="toggleMobileSidebar">
            {{ mobileSidebarOpen ? 'Close Sidebar' : 'Open Sidebar' }}
          </button>
          <button type="button" class="mobile-panel-toggle" :disabled="!hasInspectorSelection" @click="toggleMobileInspector">
            {{ mobileInspectorOpen ? 'Close Inspector' : 'Open Inspector' }}
          </button>
        </div>
        <TemplateGallery v-if="showFullScreenEmpty" />
        <template v-else>
          <PipelineComposer
            :design-mode="designMode"
            :local-definition="localDefinition"
            :view="currentView"
            @update:local-definition="handleDefinitionUpdate"
            @node-selected="handleNodeSelected"
            @edge-selected="handleEdgeSelected"
          />
        </template>
        <Transition name="pipeline-flyout">
          <PipelineFlyout
            v-if="isPipelineFlyoutOpen"
            :connection-error="connectionError"
            :show-error-state="showErrorState"
            :show-loading-state="showLoadingState"
            @close="handlePipelineSelected"
            @new-pipeline="handleNewPipeline"
            @retry="handleRetryLoad"
          />
        </Transition>
        <div v-if="!showFullScreenEmpty" class="inspector-overlay">
          <section class="inspector-panel" :class="{ 'is-visible': isInspectorVisible }" :aria-hidden="!isInspectorVisible">
            <div v-if="isNarrowViewport" class="mobile-inspector-header">
              <span>Inspector</span>
              <button type="button" class="mobile-inspector-close" @click="mobileInspectorOpen = false">Close</button>
            </div>
            <PipelineInspector :selected-node-id="selectedNodeId" :selected-edge="selectedEdge" />
          </section>
        </div>
      </section>
      <section v-if="!showFullScreenEmpty" class="runner-panel">
        <div class="runner-panel-header">
          <button
            v-if="hasLoadedPipeline && runnerExpanded"
            type="button"
            class="btn btn-primary"
            :disabled="graftSubmitting"
            @click="graftDialogOpen = true"
          >
            Graft Nodes
          </button>
          <button
            v-if="hasLoadedPipeline && runnerExpanded"
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
            :aria-expanded="runnerExpanded"
            :title="runnerExpanded ? 'Collapse panel' : 'Expand panel'"
            @click="runnerExpanded = !runnerExpanded"
          >
            {{ runnerExpanded ? '▼' : '▲' }}
          </button>
        </div>
        <PipelineRunner :collapsed="!runnerExpanded" />
      </section>
      <button
        v-if="isPipelineFlyoutOpen"
        type="button"
        class="flyout-backdrop"
        tabindex="-1"
        aria-label="Close pipelines flyout"
        @click="handleFlyoutBackdropClick"
        @contextmenu="handleFlyoutBackdropContextMenu"
      />
      <button v-if="isNarrowViewport && mobileSidebarOpen" type="button" class="mobile-overlay" aria-label="Close sidebar" @click="mobileSidebarOpen = false" />
      <button v-if="isNarrowViewport && mobileInspectorOpen" type="button" class="mobile-overlay" aria-label="Close inspector" @click="mobileInspectorOpen = false" />
    </div>
    <GraftNodesDialog
      v-if="graftDialogOpen && currentView"
      :busy="graftSubmitting"
      :existing-node-ids="existingNodeIds"
      :workflows="workflows"
      @cancel="graftDialogOpen = false"
      @submit="handleGraftSubmit"
    />
    <JoinPipelineDialog v-if="joinDialogOpen && currentView" :busy="joinSubmitting" @cancel="joinDialogOpen = false" @submit="handleJoinSubmit" />
    <SemanticZoomOverlay />
  </div>
</template>

<style scoped>
@import './PipelineView.css';
</style>
