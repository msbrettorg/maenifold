<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import DesignNodeConfigModal from './DesignNodeConfigModal.vue';
import GraftNodesDialog from './GraftNodesDialog.vue';
import JoinPipelineDialog from './JoinPipelineDialog.vue';
import PipelineComposer from './PipelineComposer.vue';
import PipelineFlyout from './PipelineFlyout.vue';
import PipelineIconRail from './PipelineIconRail.vue';
import PipelineActionBar from './PipelineActionBar.vue';
import PipelineRunner from './PipelineRunner.vue';
import PipelineSidebar from './PipelineSidebar.vue';
import PromotionDialog from './PromotionDialog.vue';
import SemanticZoomOverlay from './SemanticZoomOverlay.vue';
import TemplateGallery from './TemplateGallery.vue';
import { useFlyout } from '../composables/useFlyout';
import { usePipelineStore } from '../stores/pipeline';
import { usePipelineDialogs } from '../composables/usePipelineDialogs';
import { usePipelineLoading } from '../composables/usePipelineLoading';
import { usePipelineMobile } from '../composables/usePipelineMobile';
import type { PipelineDefinition, ViewNode } from '../lib/pipeline-types';

type SelectedEdge = { source: string; target: string } | null;
type PipelineIconRailExpose = { focusPipelineToggle: () => void };

const store = usePipelineStore();
const { currentPipelineId, currentView, designMode, error, localDefinition, successMessage, workflows } = storeToRefs(store);
const selectedNodeId = ref<string | null>(null);
const selectedEdge = ref<SelectedEdge>(null);
const runnerExpanded = ref(false);
const iconRailRef = ref<PipelineIconRailExpose | null>(null);
const configModalNodeId = ref<string | null>(null);
const configModalEdge = ref<SelectedEdge>(null);
const isConfigModalOpen = computed(() => designMode.value && (configModalNodeId.value !== null || configModalEdge.value !== null));
const { connectionError, showLoadingState, showErrorState, handleRetryLoad } = usePipelineLoading();
const { isNarrowViewport, mobileSidebarOpen, toggleMobileSidebar } = usePipelineMobile();
const { graftDialogOpen, graftSubmitting, joinDialogOpen, joinSubmitting, handleGraftSubmit, handleJoinSubmit } = usePipelineDialogs();
const { activeFlyout, close: closeFlyout, toggle: toggleFlyout } = useFlyout();
const promotionNode = ref<ViewNode | null>(null);
const isPipelineFlyoutOpen = computed(() => !isNarrowViewport.value && activeFlyout.value === 'pipelines');
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
  closeConfigModal();
  store.newPipeline();
  closePipelineFlyout();
  if (isNarrowViewport.value) {
    mobileSidebarOpen.value = false;
  }
}
function handleNodeDoubleClick(nodeId: string) {
  configModalNodeId.value = nodeId;
  configModalEdge.value = null;
}
function closeConfigModal() {
  configModalNodeId.value = null;
  configModalEdge.value = null;
}
function handleOpenGraft() {
  if (hasLoadedPipeline.value) graftDialogOpen.value = true;
}
function handleOpenPromotion() {
  const node = currentView.value?.nodes.find((n) => n.status === 'completed' && n.output);
  if (node) promotionNode.value = node;
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
}

onMounted(() => window.addEventListener('keydown', handleWindowKeyDown));
onBeforeUnmount(() => window.removeEventListener('keydown', handleWindowKeyDown));
</script>

<template>
  <div class="pipeline-view">
    <div v-if="successMessage" class="success-banner" role="status">
      <span class="success-icon">✓</span>
      <span class="success-text">{{ successMessage }}</span>
      <button class="banner-dismiss" type="button" aria-label="Dismiss success" @click="store.clearSuccess()">✕</button>
    </div>
    <div
      class="pipeline-grid"
      :class="{
        'grid-empty': showFullScreenEmpty,
        'runner-expanded': runnerExpanded,
        'mobile-sidebar-open': mobileSidebarOpen,
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
        </div>
        <TemplateGallery v-if="showFullScreenEmpty" />
        <template v-else>
          <PipelineComposer
            :design-mode="designMode"
            :local-definition="localDefinition"
            :view="currentView"
            @update:local-definition="handleDefinitionUpdate"
            @node-selected="handleNodeSelected"
            @node-double-click="handleNodeDoubleClick"
            @edge-selected="handleEdgeSelected"
            @open-graft="handleOpenGraft"
            @open-promotion="handleOpenPromotion"
          >
            <template #toolbar-actions>
              <PipelineActionBar />
            </template>
          </PipelineComposer>
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
    <PromotionDialog
      v-if="promotionNode && currentView?.pipelineId"
      :node="promotionNode"
      :pipeline-id="currentView.pipelineId"
      @close="promotionNode = null"
    />
    <DesignNodeConfigModal
      v-if="isConfigModalOpen"
      :selected-node-id="configModalNodeId"
      :selected-edge="configModalEdge"
      @close="closeConfigModal"
    />
    <SemanticZoomOverlay />
  </div>
</template>

<style scoped>
@import './PipelineView.css';
</style>
