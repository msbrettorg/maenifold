<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import type { EdgeType, PipelineEdge } from '../lib/pipeline-types';

interface JoinEdgeRow {
  source: string;
  target: string;
  type: EdgeType;
}

const props = defineProps<{
  busy?: boolean;
}>();

const emit = defineEmits<{
  (e: 'cancel'): void;
  (e: 'submit', sourceSessionId: string, edges: PipelineEdge[]): void;
}>();

const sourceSessionId = ref('');
const edgeRows = ref<JoinEdgeRow[]>([{ source: '', target: '', type: 'success' }]);
const formError = ref('');
const busy = computed(() => props.busy ?? false);

const canSubmit = computed(() =>
  !busy.value &&
  sourceSessionId.value.trim().length > 0 &&
  edgeRows.value.length > 0 &&
  !edgeRows.value.some((edge) => !edge.source.trim() || !edge.target.trim()),
);

function addEdgeRow() {
  edgeRows.value.push({ source: '', target: '', type: 'success' });
}

function removeEdgeRow(index: number) {
  if (edgeRows.value.length <= 1) return;
  edgeRows.value.splice(index, 1);
}

function normalizeEdges(rows: JoinEdgeRow[]): PipelineEdge[] {
  return rows.map((row) => ({
    source: row.source.trim(),
    target: row.target.trim(),
    type: row.type,
  }));
}

function handleSubmit() {
  formError.value = '';
  const normalizedSourceSessionId = sourceSessionId.value.trim();
  if (!normalizedSourceSessionId) {
    formError.value = 'Source pipeline session ID is required.';
    return;
  }
  const normalizedEdges = normalizeEdges(edgeRows.value);
  const hasEmptyEdge = normalizedEdges.some((edge) => edge.source.length === 0 || edge.target.length === 0);
  if (hasEmptyEdge) {
    formError.value = 'Every edge row must include source and target node IDs.';
    return;
  }

  emit('submit', normalizedSourceSessionId, normalizedEdges);
}
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && emit('cancel')">
    <DialogContent class="join-dialog panel gap-0 p-0 sm:max-w-[52rem]" :show-close-button="false">
      <DialogHeader class="dialog-header">
        <div class="dialog-heading">
          <DialogTitle>Join Pipeline</DialogTitle>
          <DialogDescription>Merge another pipeline session into this one by mapping source nodes to target nodes.</DialogDescription>
        </div>
        <DialogClose as-child>
          <button type="button" class="btn btn-ghost" :disabled="busy">Close</button>
        </DialogClose>
      </DialogHeader>

      <form class="dialog-form" @submit.prevent="handleSubmit">
        <label>
          <span>Source Pipeline Session ID</span>
          <input v-model="sourceSessionId" class="field-input" type="text" placeholder="session-abc123" />
        </label>

        <div class="edge-list">
          <div class="edge-list-header">
            <h4>Edge mappings</h4>
            <button type="button" class="btn btn-ghost" :disabled="busy" @click="addEdgeRow">Add Edge</button>
          </div>

          <div v-for="(edge, index) in edgeRows" :key="`join-edge-${index}`" class="edge-row">
            <label>
              <span>Source Node ID</span>
              <input v-model="edge.source" class="field-input" type="text" placeholder="source-node-id" />
            </label>
            <label>
              <span>Target Node ID</span>
              <input v-model="edge.target" class="field-input" type="text" placeholder="target-node-id" />
            </label>
            <label>
              <span>Edge Type</span>
              <select v-model="edge.type" class="field-input">
                <option value="success">Success</option>
                <option value="always">Always</option>
              </select>
            </label>
            <button type="button" class="btn btn-ghost edge-remove" :disabled="busy || edgeRows.length <= 1" @click="removeEdgeRow(index)">
              Remove
            </button>
          </div>
        </div>

        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

        <DialogFooter class="dialog-actions">
          <DialogClose as-child>
            <button type="button" class="btn btn-ghost" :disabled="busy">Cancel</button>
          </DialogClose>
          <button type="submit" class="btn btn-primary" :disabled="!canSubmit">
            {{ busy ? 'Joining…' : 'Join Pipeline' }}
          </button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.join-dialog {
  width: min(52rem, 100%);
  max-height: min(90vh, 52rem);
  overflow: auto;
}

.dialog-heading {
  display: grid;
  gap: 0.35rem;
  text-align: left;
}

.dialog-header {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.5rem 1.5rem 0;
}

.dialog-form,
label {
  display: grid;
  gap: 0.35rem;
}

.dialog-form {
  gap: 1rem;
  padding: 0 1.5rem 1.5rem;
}

label {
  min-width: 0;
  color: var(--muted-foreground);
  font-size: var(--text-sm);
}

.edge-list {
  display: grid;
  gap: 0.75rem;
}

.edge-list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.edge-list-header h4 {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--muted-foreground);
}

.edge-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr)) auto;
  gap: 0.75rem;
  align-items: end;
}

.edge-remove {
  justify-self: end;
}

.form-error {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--color-status-abandoned);
}

.dialog-actions {
  padding-top: 0.5rem;
}
</style>
