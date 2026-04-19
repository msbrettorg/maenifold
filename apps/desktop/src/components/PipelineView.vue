<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { ChevronDown, ChevronUp, X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import DesignNodeConfigModal from './DesignNodeConfigModal.vue';
import GraftNodesDialog from './GraftNodesDialog.vue';
import JoinPipelineDialog from './JoinPipelineDialog.vue';
import PipelineComposer from './PipelineComposer.vue';
import PipelineFlyout from './PipelineFlyout.vue';
import PipelineIconRail from './PipelineIconRail.vue';
import PipelineActionBar from './PipelineActionBar.vue';
import PipelineRunner from './PipelineRunner.vue';
import PipelineSidebar from './PipelineSidebar.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import PromotionDialog from './PromotionDialog.vue';
import SemanticZoomOverlay from './SemanticZoomOverlay.vue';
import TemplateGallery from './TemplateGallery.vue';
import { useFlyout } from '../composables/useFlyout';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { usePipelineStore } from '../stores/pipeline';
import { usePipelineDialogs } from '../composables/usePipelineDialogs';
import { usePipelineLoading } from '../composables/usePipelineLoading';
import { usePipelineMobile } from '../composables/usePipelineMobile';
import { useProvenanceStore } from '../stores/provenance';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import type { PipelineDefinition, ViewNode } from '../lib/pipeline-types';
import './PipelineView.css';

type SelectedEdge = { source: string; target: string } | null;
type PipelineIconRailExpose = { focusPipelineToggle: () => void };

const store = usePipelineStore();
const provenance = useProvenanceStore();
const zoomStore = useSemanticZoomStore();
const { navigateTo } = useProvenanceNavigation();
const { currentPipelineId, currentView, designMode, error, localDefinition, successMessage, workflows } = storeToRefs(store);
const selectedNodeId = ref<string | null>(null);
const selectedEdge = ref<SelectedEdge>(null);
const runnerExpanded = ref(false);
const iconRailRef = ref<PipelineIconRailExpose | null>(null);
const configModalNodeId = ref<string | null>(null);
const configModalEdge = ref<SelectedEdge>(null);
const isConfigModalOpen = computed(() => configModalNodeId.value !== null || configModalEdge.value !== null);
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
const showBrowseExistingEntry = computed(() =>
  !designMode.value
  && !currentView.value
);
const showTemplateGallery = computed(() => showFullScreenEmpty.value || showBrowseExistingEntry.value);
function firstNonEmptyId(...values: Array<string | null | undefined>): string | null {
  for (const value of values) {
    if (typeof value !== 'string') {
      continue;
    }
    const trimmed = value.trim();
    if (trimmed.length > 0) {
      return trimmed;
    }
  }
  return null;
}

const selectedNode = computed(() =>
  currentView.value?.nodes.find((node) => node.nodeId === selectedNodeId.value) ?? null);
const selectedNodeProvenanceDetail = computed(() => selectedNode.value?.provenance?.detail ?? null);
const currentPipelineLineageId = computed(() =>
  firstNonEmptyId(
    selectedNodeProvenanceDetail.value?.lineagePipelineRunId,
    selectedNodeProvenanceDetail.value?.lineage_pipeline_run_id,
    currentView.value?.pipelineId,
    currentPipelineId.value,
  ));
const pipelineLineageLinks = computed(() =>
  provenance.buildLineageLinks({
    gate: firstNonEmptyId(
      selectedNodeProvenanceDetail.value?.lineageGateTaskId,
      selectedNodeProvenanceDetail.value?.lineage_gate_task_id,
    ),
    pipeline: currentPipelineLineageId.value,
    thinking: firstNonEmptyId(
      selectedNodeProvenanceDetail.value?.lineageThinkingSessionId,
      selectedNodeProvenanceDetail.value?.lineage_thinking_session_id,
      selectedNode.value?.thinkingSessionId,
      selectedNodeProvenanceDetail.value?.thinkingSessionId,
    ),
    knowledge: firstNonEmptyId(
      selectedNodeProvenanceDetail.value?.lineageMemoryNoteId,
      selectedNodeProvenanceDetail.value?.lineage_memory_note_id,
    ),
    parentWorkflowId: firstNonEmptyId(
      selectedNodeProvenanceDetail.value?.parentWorkflowId,
      selectedNodeProvenanceDetail.value?.parent_workflow_id,
    ),
  }, {
    pipeline: currentView.value?.pipelineId ?? currentPipelineId.value ?? undefined,
  }));
const provenanceLinks = computed(() =>
  pipelineLineageLinks.value
    .filter((link) => link.kind !== 'pipeline' && link.id !== currentPipelineLineageId.value));

watch(() => error.value, (message) => { if (message) runnerExpanded.value = true; }, { immediate: true });
watch(() => String(currentView.value?.status ?? ''), (status) => {
  if (status === 'stuck' || status === 'abandoned') runnerExpanded.value = true;
}, { immediate: true });
watch(() => designMode.value && (localDefinition.value?.nodes?.length ?? 0) > 0, (hasContent) => {
  if (hasContent) runnerExpanded.value = true;
}, { immediate: true });
watch(isNarrowViewport, (narrow) => { if (narrow) closeFlyout(); });
watch(
  [() => provenance.pendingThinkingSessionId, currentView],
  ([thinkingSessionId, view]) => {
    if (!thinkingSessionId || !view) {
      return;
    }

    const targetNode = view.nodes.find((node) =>
      node.thinkingSessionId === thinkingSessionId
      || node.provenance?.detail.thinkingSessionId === thinkingSessionId);
    if (!targetNode) {
      return;
    }

    const sourceRect = new DOMRect(
      window.innerWidth * 0.2,
      window.innerHeight * 0.2,
      window.innerWidth * 0.6,
      window.innerHeight * 0.5,
    );
    zoomStore.enterInspect(targetNode.nodeId, sourceRect);
    provenance.clearQueuedThinkingSessionSelection();
  },
  { immediate: true },
);

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
function handleRunnerToggleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.stopPropagation();
  }
}
function handleEdgeDoubleClick(edge: { source: string; target: string }) {
  configModalEdge.value = edge;
  configModalNodeId.value = null;
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
 function handleWindowKeyDown(event: KeyboardEvent) {
   if (event.key !== 'Escape') return;
   if (isPipelineFlyoutOpen.value) {
    event.preventDefault();
    closePipelineFlyout({ restoreFocus: true });
    return;
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleWindowKeyDown);
  store.resumePollingForActiveView();
});
onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleWindowKeyDown);
  store.stopPolling();
});
</script>

<template>
  <div class="pipeline-view">
    <ProvenanceBreadcrumbs
      current-kind="pipeline"
      :current-id="currentPipelineLineageId"
      :current-label="currentView?.pipelineId ?? currentPipelineId ?? undefined"
      :links="provenanceLinks"
      @navigate="navigateTo"
    />
    <div v-if="successMessage" class="success-banner" role="status">
      <span class="success-icon">✓</span>
      <span class="success-text">{{ successMessage }}</span>
      <Button class="banner-dismiss" type="button" variant="ghost" size="icon" aria-label="Dismiss success" @click="store.clearSuccess()">
        <X class="size-4" />
      </Button>
    </div>
    <div
      class="pipeline-grid"
      :class="{
        'grid-empty': showTemplateGallery,
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
          <Button type="button" class="mobile-panel-toggle" variant="outline" size="sm" @click="toggleMobileSidebar">
            {{ mobileSidebarOpen ? 'Close Sidebar' : 'Open Sidebar' }}
          </Button>
        </div>
        <div v-if="showTemplateGallery" class="pipeline-entry-surface">
          <Button
            v-if="showBrowseExistingEntry"
            type="button"
            variant="ghost"
            class="browse-existing-btn"
            @click="handleTogglePipelineFlyout"
          >
            Browse existing pipelines
          </Button>
          <TemplateGallery />
        </div>
        <template v-else>
          <PipelineComposer
            :design-mode="designMode"
            :local-definition="localDefinition"
            :view="currentView"
            @update:local-definition="handleDefinitionUpdate"
            @node-selected="handleNodeSelected"
            @node-double-click="handleNodeDoubleClick"
            @edge-double-click="handleEdgeDoubleClick"
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
      <section v-if="!showTemplateGallery" class="runner-panel">
        <div class="runner-panel-header">
          <Button
            v-if="hasLoadedPipeline && runnerExpanded"
            type="button"
            :disabled="graftSubmitting"
            @click="graftDialogOpen = true"
          >
            Graft nodes
          </Button>
          <Button
            v-if="hasLoadedPipeline && runnerExpanded"
            type="button"
            variant="ghost"
            :disabled="joinSubmitting"
            @click="joinDialogOpen = true"
          >
            Join pipeline
          </Button>
          <Button
            class="runner-expand-btn"
            type="button"
            variant="ghost"
            aria-label="Toggle runner"
            :aria-expanded="runnerExpanded"
            :title="runnerExpanded ? 'Hide runner' : 'Show runner'"
            @click="runnerExpanded = !runnerExpanded"
            @keydown="handleRunnerToggleKeydown"
          >
            <span>Runner</span>
            <ChevronDown v-if="runnerExpanded" aria-hidden="true" class="size-4" />
            <ChevronUp v-else aria-hidden="true" class="size-4" />
          </Button>
        </div>
        <PipelineRunner :collapsed="!runnerExpanded" />
      </section>
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
