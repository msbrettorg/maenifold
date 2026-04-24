<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { ChevronDown, ChevronUp, X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent } from '@/components/ui/sheet';
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
import { usePipelineViewState } from '../composables/usePipelineViewState';
import { useProvenanceStore } from '../stores/provenance';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import type { PipelineDefinition, ViewNode } from '../lib/pipeline-types';
type PipelineIconRailExpose = { focusPipelineToggle: () => void };
const store = usePipelineStore();
const provenance = useProvenanceStore();
const zoomStore = useSemanticZoomStore();
const { navigateTo } = useProvenanceNavigation();
const { currentPipelineId, currentView, designMode, error, localDefinition, successMessage, workflows } = storeToRefs(store);
const runnerExpanded = ref(false);
const iconRailRef = ref<PipelineIconRailExpose | null>(null);
const { connectionError, showLoadingState, showErrorState, handleRetryLoad } = usePipelineLoading();
const { isNarrowViewport, mobileSidebarOpen, toggleMobileSidebar } = usePipelineMobile();
const { graftDialogOpen, graftSubmitting, joinDialogOpen, joinSubmitting, handleGraftSubmit, handleJoinSubmit } = usePipelineDialogs();
const { activeFlyout, close: closeFlyout, toggle: toggleFlyout } = useFlyout();
const promotionNode = ref<ViewNode | null>(null);
const isPipelineFlyoutOpen = computed(() => !isNarrowViewport.value && activeFlyout.value === 'pipelines');
const hasLoadedPipeline = computed(() => Boolean(currentView.value?.pipelineId));
const activePipelineLabel = computed(() => currentView.value?.pipelineId ?? currentPipelineId.value ?? '');
const showFullScreenEmpty = computed(() =>
  designMode.value && !currentView.value && (localDefinition.value?.nodes?.length ?? 0) === 0);
const showBrowseExistingEntry = computed(() => !designMode.value && !currentView.value);
const showTemplateGallery = computed(() => showFullScreenEmpty.value || showBrowseExistingEntry.value);
const {
  configModalNodeId, configModalEdge, isConfigModalOpen, existingNodeIds,
  currentPipelineLineageId, provenanceLinks,
  handleNodeSelected, handleEdgeSelected, clearInspectorSelection,
  handleNodeDoubleClick, handleEdgeDoubleClick, closeConfigModal,
} = usePipelineViewState({ currentView, currentPipelineId });
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
function focusRailToggle() {
  nextTick(() => iconRailRef.value?.focusPipelineToggle());
}
function closePipelineFlyout(options?: { restoreFocus?: boolean }) {
  if (!isPipelineFlyoutOpen.value) return;
  closeFlyout();
  if (options?.restoreFocus) focusRailToggle();
}
function handleTogglePipelineFlyout() { toggleFlyout('pipelines'); }
function handlePipelineSelected() {
  clearInspectorSelection();
  closePipelineFlyout();
}
function handleNewPipeline() {
  clearInspectorSelection();
  closeConfigModal();
  store.newPipeline();
  closePipelineFlyout();
  if (isNarrowViewport.value) mobileSidebarOpen.value = false;
}
function handleRunnerToggleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') event.stopPropagation();
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
  <div class="flex h-full flex-col overflow-hidden bg-bg-app">
    <ProvenanceBreadcrumbs
      current-kind="pipeline"
      :current-id="currentPipelineLineageId"
      :current-label="currentView?.pipelineId ?? currentPipelineId ?? undefined"
      :links="provenanceLinks"
      @navigate="navigateTo"
    />
    <div v-if="successMessage" class="flex shrink-0 items-center gap-2 bg-success/15 px-4 py-3 text-foreground" role="status">
      <span class="shrink-0 text-success">✓</span>
      <span class="min-w-0 flex-1">{{ successMessage }}</span>
      <Button type="button" variant="ghost" size="icon" aria-label="Dismiss success" @click="store.clearSuccess()"><X class="size-4" /></Button>
    </div>
    <div class="relative flex min-h-0 flex-1 flex-row overflow-hidden bg-surface">
      <section
        v-if="!isNarrowViewport"
        class="relative z-10 flex w-12 shrink-0 flex-col overflow-hidden bg-surface"
      >
        <PipelineIconRail
          ref="iconRailRef"
          :active-pipeline-label="activePipelineLabel"
          :flyout-open="isPipelineFlyoutOpen"
          :is-narrow-viewport="isNarrowViewport"
          @toggle-flyout="handleTogglePipelineFlyout"
          @new-pipeline="handleNewPipeline"
        />
      </section>
      <Sheet
        v-if="isNarrowViewport"
        :open="mobileSidebarOpen"
        @update:open="(value: boolean) => mobileSidebarOpen = value"
      >
        <SheetContent side="left" class="w-72 max-w-[88vw] p-0">
          <PipelineSidebar />
        </SheetContent>
      </Sheet>
      <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        <section class="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-surface">
          <div v-if="isNarrowViewport" class="pointer-events-none absolute left-3 right-3 top-3 z-10 flex gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              class="pointer-events-auto min-h-11 min-w-11 bg-surface-container shadow-fluent-16"
              @click="toggleMobileSidebar"
            >
              {{ mobileSidebarOpen ? 'Close Sidebar' : 'Open Sidebar' }}
            </Button>
          </div>
          <div v-if="showTemplateGallery" class="flex h-full min-h-0 flex-col gap-3 px-4 pb-3 pt-4">
            <Button
              v-if="showBrowseExistingEntry"
              type="button"
              variant="ghost"
              size="sm"
              class="self-start"
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
        <section
          v-if="!showTemplateGallery"
          class="relative flex shrink-0 flex-col overflow-hidden bg-surface transition-[max-height] duration-200 ease-out"
          :class="runnerExpanded ? 'max-h-[50vh]' : 'max-h-9'"
        >
          <div class="absolute right-3 top-3 z-10 flex items-center gap-2">
            <Button
              v-if="hasLoadedPipeline && runnerExpanded"
              type="button"
              size="sm"
              :disabled="graftSubmitting"
              @click="graftDialogOpen = true"
            >
              Graft nodes
            </Button>
            <Button
              v-if="hasLoadedPipeline && runnerExpanded"
              type="button"
              variant="ghost"
              size="sm"
              :disabled="joinSubmitting"
              @click="joinDialogOpen = true"
            >
              Join pipeline
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-label="Toggle runner"
              :aria-expanded="runnerExpanded"
              :title="runnerExpanded ? 'Hide runner' : 'Show runner'"
              class="bg-surface-high text-xs hover:bg-surface-highest"
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
      </div>
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
