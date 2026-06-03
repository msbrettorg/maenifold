<script setup lang="ts">
// Join dialog (T-600.5). Teleported into `#pipeline-overlay`. Merges a SOURCE
// pipeline's nodes/edges into the LIVE (target) pipeline via store.joinPipeline,
// optionally adding cross-pipeline linking edges. The source's full node list is
// fetched on selection (the list rows are summaries only). Node-id collisions are
// preflighted — the daemon rejects them (handler_surgery_join.rs:54-64) — and the
// source must differ from the target.
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { Merge } from '@lucide/vue';
import { usePipelinesStore } from '../../../stores/pipelines';
import { useSurgeryController } from './useSurgeryController';
import { useSurgeryActions } from './useSurgeryActions';
import SurgeryEdgeEditor, { type NodeOption } from './SurgeryEdgeEditor.vue';
import { type EdgeDraft } from '../../../lib/surgeryEdges';
import { canJoin } from '../../../lib/surgeryEnablement';
import { preflightJoin } from '../../../lib/joinPreflight';
import {
  viewNodeDisplayLabel,
  type PipelineView,
  type ViewNode,
} from '../../../lib/pipeline-types';

const store = usePipelinesStore();
const { pipelines, selectedNodes, selectedPipelineId, selectedView } = storeToRefs(store);
const controller = useSurgeryController();
const actions = useSurgeryActions();

const open = computed(() => controller.dialog.value?.kind === 'join');

const sourceId = ref('');
const sourceNodes = ref<ViewNode[]>([]);
// The FULL fetched source view — needed to gate on the source pipeline's status
// (the daemon rejects a join when EITHER source or target has Ready|Active|
// Retrying nodes: handler_surgery_join.rs:133-150). T-600.5 Defect 2.
const sourceView = ref<PipelineView | null>(null);
const loadingSource = ref(false);
const edges = ref<EdgeDraft[]>([]);
const localError = ref<string | null>(null);

// Target (live selection) gate — reactive, so it tracks polling updates.
const targetBlock = computed(() => canJoin(selectedView.value));
// Source gate — only meaningful once a source view is loaded for the CURRENTLY
// selected source id. A null/failed/stale view does NOT count as enabled (a
// failed fetch must not let a join proceed with no source-status preflight).
const sourceLoaded = computed(
  () => sourceView.value !== null && sourceView.value.pipelineId === sourceId.value
);
const sourceBlock = computed(() =>
  sourceLoaded.value ? canJoin(sourceView.value) : { enabled: false as const }
);
const joinBlockReason = computed(
  () => (!targetBlock.value.enabled ? targetBlock.value.reason : sourceBlock.value.reason) ?? null
);

watch(open, (isOpen) => {
  if (!isOpen) return;
  sourceId.value = '';
  sourceNodes.value = [];
  sourceView.value = null;
  edges.value = [];
  localError.value = null;
});

// Candidate sources = every other live pipeline (exclude the current target).
const candidates = computed(() =>
  pipelines.value.filter((row) => row.pipelineId !== selectedPipelineId.value)
);

const targetIds = computed(() => selectedNodes.value.map((node) => node.nodeId));

const nodeOptions = computed<NodeOption[]>(() => {
  const target: NodeOption[] = selectedNodes.value.map((node) => ({
    id: node.nodeId,
    label: `${viewNodeDisplayLabel(node)} (target)`,
  }));
  const source: NodeOption[] = sourceNodes.value.map((node) => ({
    id: node.nodeId,
    label: `${viewNodeDisplayLabel(node)} (source)`,
  }));
  return [...target, ...source];
});

async function onSourcePicked(): Promise<void> {
  localError.value = null;
  sourceNodes.value = [];
  sourceView.value = null;
  edges.value = [];
  const requestedId = sourceId.value;
  if (!requestedId) return;
  loadingSource.value = true;
  try {
    const view = await store.fetchViewRaw(requestedId);
    // Race guard: if the user changed the source pick while this fetch was in
    // flight, ignore the late result so it can't seat the wrong source view.
    if (sourceId.value !== requestedId) return;
    sourceView.value = view;
    sourceNodes.value = view.nodes;
  } catch (err) {
    if (sourceId.value !== requestedId) return;
    localError.value = err instanceof Error ? err.message : String(err);
  } finally {
    if (sourceId.value === requestedId) loadingSource.value = false;
  }
}

async function submit(): Promise<void> {
  localError.value = null;
  // All submit-time validation — including the LIVE source re-fetch and the
  // source/target status re-check — lives in the pure `preflightJoin` helper so
  // it can be unit-tested. We must not trust the snapshot taken at selection
  // time; the source may have started running since (handler_surgery_join.rs:
  // 133-150). T-600.5 Defect 2.
  const pre = await preflightJoin({
    sourceId: sourceId.value,
    targetId: selectedPipelineId.value,
    targetView: selectedView.value,
    targetNodeIds: targetIds.value,
    edgeDrafts: edges.value,
    fetchSource: (id) => store.fetchViewRaw(id),
  });
  if (!pre.ok) {
    localError.value = pre.error;
    return;
  }
  // Seat the freshly-fetched source so the UI reflects the authoritative state.
  sourceView.value = pre.sourceView;
  sourceNodes.value = pre.sourceView.nodes;
  const result = await actions.join(sourceId.value, pre.edges);
  if (result.ok) controller.closeDialog();
}

function cancel(): void {
  controller.closeDialog();
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open" class="dlg-backdrop" @click.self="cancel">
      <section class="dlg-panel" role="dialog" aria-label="Join pipeline">
        <header class="dlg-head">
          <h2><Merge :size="15" :stroke-width="2" /> Join pipeline</h2>
        </header>

        <div class="dlg-body">
          <label class="dlg-field">
            <span class="dlg-label">Source pipeline</span>
            <select v-model="sourceId" class="dlg-input" @change="onSourcePicked">
              <option value="">Select a pipeline…</option>
              <option v-for="row in candidates" :key="row.pipelineId" :value="row.pipelineId">
                {{ row.name ?? row.pipelineId }} ({{ row.status }})
              </option>
            </select>
          </label>

          <p v-if="loadingSource" class="dlg-hint">Loading source nodes…</p>
          <p v-else-if="sourceNodes.length > 0" class="dlg-hint">
            {{ sourceNodes.length }} source node(s) will be merged in.
          </p>

          <p v-if="joinBlockReason" class="dlg-hint dlg-block" role="alert">
            {{ joinBlockReason }}
          </p>

          <section v-if="sourceNodes.length > 0" class="dlg-field">
            <span class="dlg-label">Cross-pipeline edges (optional)</span>
            <SurgeryEdgeEditor v-model="edges" :node-options="nodeOptions" />
          </section>

          <p class="dlg-hint">
            Joining copies every source node and edge into this pipeline. The
            source pipeline is left unchanged.
          </p>
        </div>

        <footer class="dlg-foot">
          <span v-if="localError || actions.lastError.value" class="dlg-error" role="alert">
            {{ localError ?? actions.lastError.value }}
          </span>
          <span class="dlg-spacer" />
          <button type="button" class="dlg-btn" @click="cancel">Cancel</button>
          <button
            type="button"
            class="dlg-btn primary"
            :disabled="
              actions.busy.value ||
              !sourceId ||
              loadingSource ||
              !targetBlock.enabled ||
              !sourceBlock.enabled
            "
            @click="submit"
          >
            Join
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.dlg-backdrop {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--bg-main) 30%, transparent);
  z-index: 22;
}

.dlg-panel {
  display: flex;
  flex-direction: column;
  width: min(580px, 94%);
  max-height: 90%;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-main);
  box-shadow: 0 14px 40px rgba(0, 0, 0, 0.32);
  overflow: hidden;
}

.dlg-head {
  padding: 0.7rem 0.9rem;
  border-bottom: 1px solid var(--border-color);
}

.dlg-head h2 {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
  font-size: 0.92rem;
  color: var(--text-primary);
}

.dlg-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.8rem 0.9rem;
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.dlg-field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.dlg-label {
  font-size: 0.74rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.dlg-input {
  width: 100%;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  outline: none;
}

.dlg-hint {
  margin: 0;
  font-size: 0.74rem;
  color: var(--text-muted);
  line-height: 1.45;
}

.dlg-block {
  color: var(--bg-danger, #d9534f);
  font-weight: 600;
}

.dlg-foot {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 0.9rem;
  border-top: 1px solid var(--border-color);
}

.dlg-spacer {
  flex: 1;
}

.dlg-error {
  font-size: 0.74rem;
  color: var(--bg-danger, #d9534f);
}

.dlg-btn {
  padding: 0.4rem 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}

.dlg-btn:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.dlg-btn:disabled {
  cursor: default;
  opacity: 0.5;
}

.dlg-btn.primary {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 40%, transparent);
}
</style>
