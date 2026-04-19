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
      <Button v-if="isNarrowViewport && mobileSidebarOpen" variant="ghost" type="button" class="mobile-overlay" aria-label="Close sidebar" @click="mobileSidebarOpen = false" />
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
.pipeline-view {
  --rail-width: 48px;
  --flyout-width: 320px;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--background);
}
.error-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: color-mix(in srgb, var(--background) 82%, var(--terracotta) 18%);
  color: var(--foreground);
  flex-shrink: 0;
}
.error-icon { flex-shrink: 0; }
.error-text { min-width: 0; }
.success-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: color-mix(in srgb, var(--background) 84%, var(--sage) 16%);
  color: var(--foreground);
  flex-shrink: 0;
}
.success-icon { flex-shrink: 0; color: var(--color-status-completed); }
.success-text { min-width: 0; flex: 1; }
.banner-dismiss {
  border: 0;
  padding: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.pipeline-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: var(--rail-width) minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr) auto;
  grid-template-areas:
    'rail composer'
    'rail runner';
  transition: grid-template-rows 0.25s ease;
  overflow: hidden;
  position: relative;
  background: var(--surface);
}
.pipeline-grid.grid-empty {
  grid-template-rows: minmax(0, 1fr);
  grid-template-areas: 'rail composer';
}
.pipeline-grid.runner-expanded .runner-panel { max-height: 50vh; }
.rail-panel,
.sidebar-panel,
.composer-panel,
.runner-panel {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--surface);
}
.rail-panel {
  grid-area: rail;
  position: relative;
  z-index: 12;
  background: var(--surface);
}
.sidebar-panel {
  grid-area: rail;
  background: var(--surface);
}
.composer-panel {
  grid-area: composer;
  position: relative;
}
.pipeline-entry-surface {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  height: 100%;
  min-height: 0;
  padding: 1rem 1rem 0.75rem;
}
.browse-existing-btn {
  align-self: flex-start;
  height: 2rem;
  border-radius: 0.25rem;
}
.runner-panel {
  grid-area: runner;
  position: relative;
  max-height: 36px;
  transition: max-height 0.25s ease;
  background: var(--surface);
}
.grid-empty .runner-panel { display: none; }
.runner-panel-header {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.runner-expand-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  min-height: 2rem;
  padding: 0 0.6rem;
  border: 0;
  border-radius: 0.25rem;
  background: var(--surface-high);
  color: var(--text);
  font: inherit;
  font-size: var(--text-xs);
  cursor: pointer;
}
.runner-expand-btn:hover {
  background: var(--surface-highest);
}
.mobile-panel-controls,
.mobile-inspector-header,
.mobile-overlay {
  display: none;
}
@media (max-width: 900px) {
  .pipeline-grid {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    grid-template-areas:
      'composer'
      'runner';
  }
  .pipeline-grid.grid-empty {
    grid-template-rows: minmax(0, 1fr);
    grid-template-areas: 'composer';
  }
  .rail-panel { display: none; }
  .sidebar-panel {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: min(var(--sidebar-width), 88vw);
    min-width: min(var(--sidebar-min-width), 88vw);
    max-width: min(var(--sidebar-max-width), 88vw);
    z-index: 20;
    transform: translateX(-100%);
    transition: transform 0.2s ease;
    box-shadow: var(--shadow-16);
  }
  .pipeline-grid.mobile-sidebar-open .sidebar-panel { transform: translateX(0); }
  .composer-panel { position: relative; }
  .mobile-panel-controls {
    display: flex;
    position: absolute;
    top: 0.75rem;
    left: 0.75rem;
    right: 0.75rem;
    z-index: 8;
    gap: 0.5rem;
    pointer-events: none;
  }
  .mobile-panel-toggle {
    pointer-events: auto;
    min-height: 2.75rem;
    min-width: 2.75rem;
    padding: 0.5rem 0.875rem;
    border-radius: 0.25rem;
    background: var(--surface-container);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    box-shadow: var(--shadow-16);
  }
  .mobile-panel-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .mobile-inspector-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: var(--surface);
    font-size: var(--text-sm);
  }
  .mobile-inspector-close {
    min-height: 2.5rem;
    padding: 0.25rem 0.75rem;
    border-radius: 0.25rem;
    background: var(--surface-high);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .mobile-overlay {
    display: block;
    position: absolute;
    inset: 0;
    border: 0;
    background: color-mix(in srgb, var(--background) 40%, transparent);
    z-index: 18;
    cursor: pointer;
  }
  .pipeline-grid.mobile-inspector-open .mobile-overlay { z-index: 9; }
  .runner-panel-header {
    top: 0.5rem;
    right: 0.5rem;
  }
}
</style>
