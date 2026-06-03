<script setup lang="ts">
// Graft dialog (T-600.5). Teleported into `#pipeline-overlay`. Stages NEW nodes
// (configured through the shared, stateless NodeFormCore) plus linking edges
// (SurgeryEdgeEditor), then grafts them into the LIVE pipeline via
// store.graftNodes. Node ids are generated unique against BOTH existing live
// nodes and already-staged nodes, and collisions are preflighted (the daemon
// rejects duplicate ids: handler_surgery_graft.rs:125-132). When opened from a
// node it seeds a run-after edge from that node to the first staged node.
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { Scissors } from '@lucide/vue';
import { usePipelinesStore } from '../../../stores/pipelines';
import { useSurgeryController } from './useSurgeryController';
import { useSurgeryActions } from './useSurgeryActions';
import NodeFormCore from './NodeFormCore.vue';
import SurgeryEdgeEditor, { type NodeOption } from './SurgeryEdgeEditor.vue';
import {
  ADD_NODE_LABELS,
  constructNode,
  uniqueNodeId,
  type AddNodeKind,
} from '../../../lib/pipelineDesignOps';
import { buildEdges, collisions, type EdgeDraft } from '../../../lib/surgeryEdges';
import { canGraft } from '../../../lib/surgeryEnablement';
import { viewNodeDisplayLabel, type PipelineNode } from '../../../lib/pipeline-types';

const ADD_KINDS: AddNodeKind[] = [
  'ask-agent',
  'add-session-node',
  'run-tool',
  'think',
  'run-workflow',
];

const store = usePipelinesStore();
const { selectedNodes, selectedView } = storeToRefs(store);
const controller = useSurgeryController();
const actions = useSurgeryActions();

const open = computed(() => controller.dialog.value?.kind === 'graft');
const graftEnable = computed(() => canGraft(selectedView.value));

const staged = ref<PipelineNode[]>([]);
const edges = ref<EdgeDraft[]>([]);
const editingDraft = ref<Record<string, unknown> | null>(null);
const localError = ref<string | null>(null);
const runAfterId = ref<string | null>(null);

watch(open, (isOpen) => {
  if (!isOpen) return;
  staged.value = [];
  edges.value = [];
  editingDraft.value = null;
  localError.value = null;
  runAfterId.value = controller.dialog.value?.nodeId ?? null;
});

const existingIds = computed(() => selectedNodes.value.map((node) => node.nodeId));

const nodeOptions = computed<NodeOption[]>(() => {
  const existing: NodeOption[] = selectedNodes.value.map((node) => ({
    id: node.nodeId,
    label: `${viewNodeDisplayLabel(node)} (live)`,
  }));
  const fresh: NodeOption[] = staged.value.map((node) => ({
    id: node.id,
    label: `${node.displayLabel ?? node.id} (new)`,
  }));
  return [...existing, ...fresh];
});

function newId(): string {
  const placeholders = existingIds.value.map((id) => ({ id } as unknown as PipelineNode));
  return uniqueNodeId([...placeholders, ...staged.value]);
}

function startAdd(kind: AddNodeKind): void {
  localError.value = null;
  editingDraft.value = constructNode(kind, newId()) as unknown as Record<string, unknown>;
}

function onNodeSubmit(node: PipelineNode): void {
  staged.value = [...staged.value, node];
  // Seed a run-after edge from the originating node to the first staged node.
  if (runAfterId.value && edges.value.length === 0) {
    edges.value = [{ source: runAfterId.value, target: node.id, type: 'success' }];
  }
  editingDraft.value = null;
}

function onNodeCancel(): void {
  editingDraft.value = null;
}

function removeStaged(id: string): void {
  staged.value = staged.value.filter((node) => node.id !== id);
  edges.value = edges.value.filter((edge) => edge.source !== id && edge.target !== id);
}

async function submit(): Promise<void> {
  localError.value = null;
  // Hard-block: the daemon rejects a graft while the pipeline still has remaining
  // work (handler_surgery_graft.rs:38-43). Defense-in-depth behind the disabled
  // menu item (T-600.5 Defect 1) — state may have changed since the dialog opened.
  const graftable = canGraft(selectedView.value);
  if (!graftable.enabled) {
    localError.value = graftable.reason ?? 'Grafting is not allowed for this pipeline right now.';
    return;
  }
  if (staged.value.length === 0) {
    localError.value = 'Add at least one node to graft.';
    return;
  }
  const stagedIds = staged.value.map((node) => node.id);
  const clash = collisions(existingIds.value, stagedIds);
  if (clash.length > 0) {
    localError.value = `Node id collision with live pipeline: ${clash.join(', ')}`;
    return;
  }
  const built = buildEdges(edges.value);
  if ('error' in built) {
    localError.value = `Edge ${built.index + 1}: ${built.error}`;
    return;
  }
  const result = await actions.graft(staged.value, built.edges);
  if (result.ok) controller.closeDialog();
}

function cancel(): void {
  controller.closeDialog();
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open" class="dlg-backdrop" @click.self="cancel">
      <section class="dlg-panel" role="dialog" aria-label="Graft nodes">
        <header class="dlg-head">
          <h2><Scissors :size="15" :stroke-width="2" /> Graft nodes</h2>
          <code v-if="runAfterId" class="dlg-sub">after {{ runAfterId }}</code>
        </header>

        <div class="dlg-body">
          <p v-if="!graftEnable.enabled" class="dlg-hint dlg-block" role="alert">
            {{ graftEnable.reason }}
          </p>
          <!-- Node-config sub-panel (shared NodeFormCore). Shown while adding. -->
          <div v-if="editingDraft" class="graft-editor">
            <NodeFormCore
              :node="editingDraft ?? {}"
              submit-label="Add node"
              @submit="onNodeSubmit"
              @cancel="onNodeCancel"
            />
          </div>

          <template v-else>
            <section class="graft-section">
              <span class="dlg-label">Add a node</span>
              <div class="graft-kinds">
                <button
                  v-for="kind in ADD_KINDS"
                  :key="kind"
                  type="button"
                  class="graft-kind"
                  @click="startAdd(kind)"
                >
                  {{ ADD_NODE_LABELS[kind] }}
                </button>
              </div>
            </section>

            <section class="graft-section">
              <span class="dlg-label">Staged nodes ({{ staged.length }})</span>
              <p v-if="staged.length === 0" class="dlg-hint">No nodes staged yet.</p>
              <ul v-else class="graft-list">
                <li v-for="node in staged" :key="node.id" class="graft-item">
                  <span class="graft-item-id">{{ node.displayLabel ?? node.id }}</span>
                  <span class="graft-item-type">{{ node.type ?? 'workflow' }}</span>
                  <button type="button" class="graft-item-x" @click="removeStaged(node.id)">
                    Remove
                  </button>
                </li>
              </ul>
            </section>

            <section class="graft-section">
              <span class="dlg-label">Linking edges</span>
              <SurgeryEdgeEditor v-model="edges" :node-options="nodeOptions" />
            </section>
          </template>
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
            :disabled="actions.busy.value || !!editingDraft || staged.length === 0 || !graftEnable.enabled"
            @click="submit"
          >
            Graft {{ staged.length }} node{{ staged.length === 1 ? '' : 's' }}
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
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

.dlg-sub {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.dlg-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.8rem 0.9rem;
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.dlg-label {
  font-size: 0.74rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.dlg-hint {
  margin: 0;
  font-size: 0.74rem;
  color: var(--text-muted);
}

.dlg-block {
  color: var(--bg-danger, #d9534f);
  font-weight: 600;
}

.graft-editor {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
}

.graft-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.graft-kinds {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.graft-kind {
  padding: 0.32rem 0.55rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
}

.graft-kind:hover {
  background: var(--bg-hover);
}

.graft-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.graft-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
}

.graft-item-id {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.78rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.graft-item-type {
  font-size: 0.7rem;
  color: var(--text-muted);
  text-transform: uppercase;
}

.graft-item-x {
  padding: 0.2rem 0.45rem;
  border: 1px solid transparent;
  border-radius: 5px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.72rem;
  cursor: pointer;
}

.graft-item-x:hover {
  color: var(--bg-danger, #d9534f);
  background: var(--bg-hover);
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
