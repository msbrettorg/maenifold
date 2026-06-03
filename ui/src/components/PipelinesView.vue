<script setup lang="ts">
// Pipelines tab shell (T-600.2).
//
// Two responsibilities:
//  1. Pipeline-management chrome — an icon rail + searchable flyout list backed
//     by the `pipelines` store (`refreshList` / `viewPipeline`), giving the tab
//     operational parity with the old UI's pipeline browser.
//  2. The DAG stage — the Vue Flow canvas plus FOUR empty Teleport mount targets
//     (`#pipeline-toolbar`, `#pipeline-inspector`, `#pipeline-runner`,
//     `#pipeline-overlay`). Sibling T-600.x tasks teleport their panels into
//     these targets WITHOUT editing this shell. This task itself teleports the
//     canvas toolbar into `#pipeline-toolbar`.
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { GitBranch, PanelLeft, RefreshCw, Search } from '@lucide/vue';
import { usePipelinesStore } from '../stores/pipelines';
import { usePipelineDesignStore } from '../stores/pipelineDesign';
import { useSurgeryController } from './pipeline/surgery/useSurgeryController';
import PipelineCanvas from './pipeline/PipelineCanvas.vue';
import PipelineToolbar from './pipeline/PipelineToolbar.vue';
import NodeInspectorPane from './pipeline/NodeInspectorPane.vue';
import SemanticZoomOverlay from './pipeline/SemanticZoomOverlay.vue';
import PipelineRunner from './pipeline/runner/PipelineRunner.vue';
import PipelineContextMenu from './pipeline/design/PipelineContextMenu.vue';
import NodeConfigForm from './pipeline/design/NodeConfigForm.vue';
import EdgeConfigForm from './pipeline/design/EdgeConfigForm.vue';
import SaveStatusBadge from './pipeline/design/SaveStatusBadge.vue';
import PipelineSurgeryToolbar from './pipeline/surgery/PipelineSurgeryToolbar.vue';
import PipelineSurgeryMenu from './pipeline/surgery/PipelineSurgeryMenu.vue';
import GraftNodesDialog from './pipeline/surgery/GraftNodesDialog.vue';
import JoinPipelineDialog from './pipeline/surgery/JoinPipelineDialog.vue';
import ForkPipelineDialog from './pipeline/surgery/ForkPipelineDialog.vue';
import SurgeryStatusToast from './pipeline/surgery/SurgeryStatusToast.vue';
import TemplateGallery from './pipeline/templates/TemplateGallery.vue';
import PromoteToMemoryDialog from './pipeline/promote/PromoteToMemoryDialog.vue';

const store = usePipelinesStore();
const design = usePipelineDesignStore();
const surgeryController = useSurgeryController();
const { pipelines, loading, error, selectedPipelineId, viewLoading } = storeToRefs(store);

const flyoutOpen = ref(true);
const query = ref('');

const filteredPipelines = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) return pipelines.value;
  return pipelines.value.filter((item) => {
    const haystack = `${item.name ?? ''} ${item.pipelineId} ${item.status}`.toLowerCase();
    return haystack.includes(needle);
  });
});

onMounted(() => {
  void store.bootstrap();
});

function refresh(): void {
  void store.refreshList();
}

function selectPipeline(pipelineId: string): void {
  if (viewLoading.value) return;
  if (pipelineId === selectedPipelineId.value) return;
  // Loading another pipeline navigates away from any in-flight design edits.
  if (!design.confirmDiscardDesignChanges()) return;
  design.exitDesignMode();
  // Close any open surgery menu/dialog and drop the captured target so a stale
  // surface can't mutate the newly selected pipeline (T-600.5 Defect 3).
  surgeryController.reset();
  void store.viewPipeline(pipelineId);
}
</script>

<template>
  <section class="pipelines-view">
    <!-- Icon rail -->
    <nav class="rail" aria-label="Pipelines navigation">
      <button
        type="button"
        class="rail-btn"
        :class="{ active: flyoutOpen }"
        aria-label="Toggle pipeline list"
        title="Pipeline list"
        @click="flyoutOpen = !flyoutOpen"
      >
        <PanelLeft :size="17" :stroke-width="2" />
      </button>
      <button
        type="button"
        class="rail-btn"
        :disabled="loading"
        aria-label="Refresh pipelines"
        title="Refresh"
        @click="refresh"
      >
        <RefreshCw :size="16" :stroke-width="2" :class="{ spinning: loading }" />
      </button>
    </nav>

    <!-- Flyout: searchable pipeline list -->
    <aside v-if="flyoutOpen" class="flyout">
      <header class="flyout-header">
        <span class="flyout-title">
          <GitBranch :size="14" :stroke-width="2" />
          <strong>Pipelines</strong>
        </span>
        <span class="flyout-count">{{ pipelines.length }}</span>
      </header>

      <div class="flyout-search">
        <Search :size="14" :stroke-width="2" />
        <input
          v-model="query"
          type="search"
          class="flyout-input"
          placeholder="Search pipelines…"
          aria-label="Search pipelines"
        />
      </div>

      <p v-if="error" class="flyout-error" role="status">{{ error }}</p>

      <div class="flyout-list">
        <p v-if="loading && pipelines.length === 0" class="flyout-hint">Loading…</p>
        <p v-else-if="pipelines.length === 0" class="flyout-hint">No pipelines yet.</p>
        <p v-else-if="filteredPipelines.length === 0" class="flyout-hint">No matches.</p>
        <button
          v-for="item in filteredPipelines"
          :key="item.pipelineId"
          type="button"
          class="pipeline-row"
          :class="{ selected: item.pipelineId === selectedPipelineId }"
          @click="selectPipeline(item.pipelineId)"
        >
          <span class="pipeline-name">{{ item.name ?? item.pipelineId }}</span>
          <span class="pipeline-meta">
            <span class="pipeline-status">{{ item.status }}</span>
            <span class="pipeline-progress">{{ item.completedCount }}/{{ item.nodeCount }}</span>
          </span>
        </button>
      </div>
    </aside>

    <!-- DAG stage -->
    <div class="stage">
      <!-- Teleport mount targets. Empty until a sibling teleports a panel in. -->
      <div id="pipeline-toolbar" class="mount mount-toolbar"></div>
      <div id="pipeline-inspector" class="mount mount-inspector"></div>

      <PipelineCanvas />

      <!-- Empty-stage entry point: the template + intent gallery, shown only
           when no pipeline is selected. Instantiates REAL pipelines (T-600.8). -->
      <TemplateGallery v-if="!selectedPipelineId" class="stage-gallery" />

      <div id="pipeline-runner" class="mount mount-runner"></div>
      <div id="pipeline-overlay" class="mount mount-overlay"></div>
    </div>

    <!-- Canvas toolbar teleports into #pipeline-toolbar above. -->
    <PipelineToolbar />
    <!-- Node inspector + semantic-zoom overlay teleport into their mount
         targets (#pipeline-inspector / #pipeline-overlay) above (T-600.6). -->
    <NodeInspectorPane />
    <SemanticZoomOverlay />

    <!-- Runner panel (action bar + run form + event log + history) teleports
         into #pipeline-runner. Mounted here so its Teleport target resolves;
         it renders nothing until a pipeline is selected. -->
    <PipelineRunner />

    <!-- Design mode (T-600.4): the Save/badge/toggle teleports into
         #pipeline-toolbar; the context menu + node/edge config forms teleport
         into #pipeline-overlay. All communicate through the design store +
         design controller singleton, never props/emits across the teleport. -->
    <SaveStatusBadge />
    <PipelineContextMenu />
    <NodeConfigForm />
    <EdgeConfigForm />

    <!-- Graph surgery (T-600.5): SERVER-MODE operations on the selected LIVE
         pipeline (batch rewind, node restart/abandon, graft, join, fork). The
         parallel system to design-mode editing — suppressed while design mode is
         active. Toolbar teleports into #pipeline-toolbar; the node menu, dialogs
         and status toast teleport into #pipeline-overlay. All communicate through
         the surgery controller singleton, never props/emits across the teleport. -->
    <PipelineSurgeryToolbar />
    <PipelineSurgeryMenu />
    <GraftNodesDialog />
    <JoinPipelineDialog />
    <ForkPipelineDialog />
    <SurgeryStatusToast />

    <!-- Promote-to-memory dialog (T-600.8): launched from the SemanticZoom
         overlay via the usePromoteToMemory singleton; teleports into
         #pipeline-overlay. Renders nothing until opened. -->
    <PromoteToMemoryDialog />
  </section>
</template>

<style scoped>
.pipelines-view {
  /* Pipeline node-type accent + status design tokens. Defined here so they
     cascade into the Vue Flow node/edge components rendered inside the stage. */
  --node-accent-prompt: #adc6ff;
  --node-accent-workflow: #6aa0f5;
  --node-accent-thinking: #c4a7e7;
  --node-accent-tool: #88c98a;
  --node-status-pending: var(--text-muted);
  --node-status-active: #e0af68;
  --node-status-stuck: #ef8d54;
  --node-status-completed: #6cc070;
  --node-status-abandoned: var(--bg-danger);

  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  gap: 0.75rem;
  padding: 0.75rem;
  overflow: hidden;
}

/* Icon rail */
.rail {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
}

.rail-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.rail-btn:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.rail-btn.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.rail-btn:disabled {
  cursor: default;
  opacity: 0.5;
}

.spinning {
  animation: pipelines-spin 0.9s linear infinite;
}

@keyframes pipelines-spin {
  to {
    transform: rotate(360deg);
  }
}

/* Flyout list */
.flyout {
  flex-shrink: 0;
  width: 280px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  overflow: hidden;
}

.flyout-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.65rem 0.75rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 68%, transparent);
  color: var(--text-secondary);
}

.flyout-title {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.76rem;
  font-weight: 700;
  text-transform: uppercase;
}

.flyout-title strong {
  color: var(--text-primary);
  font-size: 0.85rem;
  text-transform: none;
}

.flyout-count {
  font-size: 0.72rem;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

.flyout-search {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0.55rem;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-main);
  color: var(--text-muted);
}

.flyout-input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  outline: none;
}

.flyout-error {
  margin: 0 0.55rem 0.5rem;
  padding: 0.45rem 0.55rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.76rem;
}

.flyout-list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0 0.45rem 0.5rem;
  overflow-y: auto;
}

.flyout-hint {
  margin: 0.5rem;
  color: var(--text-muted);
  font-size: 0.82rem;
}

.pipeline-row {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  width: 100%;
  padding: 0.45rem 0.55rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.pipeline-row:hover {
  background: var(--bg-hover);
}

.pipeline-row.selected {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.pipeline-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.84rem;
  font-weight: 550;
}

.pipeline-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.pipeline-status {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-secondary);
}

.pipeline-progress {
  font-size: 0.72rem;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

/* DAG stage */
.stage {
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  overflow: hidden;
}

/* Teleport mount targets — positioned regions that collapse while empty so an
   un-teleported target never intercepts canvas interaction. */
.mount:empty {
  display: none;
}

.mount-toolbar {
  position: absolute;
  top: 0.6rem;
  left: 0.6rem;
  z-index: 7;
}

.mount-inspector {
  position: absolute;
  top: 0.6rem;
  right: 0.6rem;
  bottom: 0.6rem;
  z-index: 7;
}

.mount-runner {
  position: absolute;
  left: 0.6rem;
  right: 0.6rem;
  bottom: 0.6rem;
  z-index: 6;
}

.mount-overlay {
  position: absolute;
  inset: 0;
  z-index: 8;
}

/* Template gallery overlays the (empty) canvas when no pipeline is selected. */
.stage-gallery {
  position: absolute;
  inset: 0;
  z-index: 5;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  overflow: hidden;
}
</style>
