<script setup lang="ts">
// Fork dialog (T-600.5). Teleported into `#pipeline-overlay`. Forks a NEW
// pipeline from a node's reachable subgraph via store.forkPipeline. Opened from
// the surgery menu (seeded with the right-clicked node) or with an empty node so
// the user can pick the fork root. An optional new-session name is forwarded; the
// daemon allocates one when blank (and rejects a name that already exists).
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { GitFork } from '@lucide/vue';
import { usePipelinesStore } from '../../../stores/pipelines';
import { useSurgeryController } from './useSurgeryController';
import { useSurgeryActions } from './useSurgeryActions';
import { viewNodeDisplayLabel } from '../../../lib/pipeline-types';

const store = usePipelinesStore();
const { selectedNodes } = storeToRefs(store);
const controller = useSurgeryController();
const actions = useSurgeryActions();

const open = computed(() => controller.dialog.value?.kind === 'fork');

const fromNodeId = ref('');
const newName = ref('');
const localError = ref<string | null>(null);

watch(open, (isOpen) => {
  if (!isOpen) return;
  fromNodeId.value = controller.dialog.value?.nodeId ?? '';
  newName.value = '';
  localError.value = null;
});

async function submit(): Promise<void> {
  localError.value = null;
  if (!fromNodeId.value) {
    localError.value = 'Pick a node to fork from.';
    return;
  }
  const result = await actions.fork(fromNodeId.value, newName.value);
  if (result.ok) controller.closeDialog();
}

function cancel(): void {
  controller.closeDialog();
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open" class="dlg-backdrop" @click.self="cancel">
      <section class="dlg-panel" role="dialog" aria-label="Fork pipeline">
        <header class="dlg-head">
          <h2><GitFork :size="15" :stroke-width="2" /> Fork pipeline</h2>
        </header>

        <div class="dlg-body">
          <label class="dlg-field">
            <span class="dlg-label">Fork from node</span>
            <select v-model="fromNodeId" class="dlg-input">
              <option value="">Select a node…</option>
              <option v-for="node in selectedNodes" :key="node.nodeId" :value="node.nodeId">
                {{ viewNodeDisplayLabel(node) }} ({{ node.status }})
              </option>
            </select>
          </label>

          <label class="dlg-field">
            <span class="dlg-label">New session name (optional)</span>
            <input
              v-model="newName"
              class="dlg-input"
              type="text"
              placeholder="auto-generated when blank"
            />
          </label>

          <p class="dlg-hint">
            Forking copies the node and everything reachable from it into a new
            pipeline. The source pipeline is left unchanged.
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
            :disabled="actions.busy.value || !fromNodeId"
            @click="submit"
          >
            Fork
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
  width: min(560px, 92%);
  max-height: 88%;
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
  gap: 0.25rem;
}

.dlg-label {
  font-size: 0.74rem;
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
