<script setup lang="ts">
import { computed, ref } from 'vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
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
    <DialogContent class="join-dialog gap-0 p-0 sm:max-w-[52rem]" :show-close-button="false">
      <DialogHeader class="dialog-header">
        <div class="dialog-heading">
          <DialogTitle>Join pipeline</DialogTitle>
          <DialogDescription>Merge another pipeline session into this one by mapping source nodes to target nodes.</DialogDescription>
        </div>
        <DialogClose as-child>
          <Button type="button" variant="ghost" :disabled="busy">Close</Button>
        </DialogClose>
      </DialogHeader>

      <form class="dialog-form" @submit.prevent="handleSubmit">
        <label>
          <span>Source pipeline session ID</span>
          <Input v-model="sourceSessionId" type="text" placeholder="session-abc123" />
        </label>

        <div class="edge-list">
          <div class="edge-list-header">
            <h4>Edge mappings</h4>
            <Button type="button" variant="ghost" :disabled="busy" @click="addEdgeRow">Add Edge</Button>
          </div>

          <div v-for="(edge, index) in edgeRows" :key="`join-edge-${index}`" class="edge-row">
            <label>
              <span>Source node ID</span>
              <Input v-model="edge.source" type="text" placeholder="source-node-id" />
            </label>
            <label>
              <span>Target node ID</span>
              <Input v-model="edge.target" type="text" placeholder="target-node-id" />
            </label>
            <label>
              <span>Edge type</span>
              <select
                v-model="edge.type"
                class="w-full bg-[var(--surface)] text-foreground border border-[rgba(66,71,83,0.15)] rounded-[4px] px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
              >
                <option value="success">Success</option>
                <option value="always">Always</option>
              </select>
            </label>
            <Button type="button" variant="ghost" class="edge-remove" :disabled="busy || edgeRows.length <= 1" @click="removeEdgeRow(index)">
              Remove
            </Button>
          </div>
        </div>

        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

        <DialogFooter class="dialog-actions">
          <DialogClose as-child>
            <Button type="button" variant="ghost" :disabled="busy">Cancel</Button>
          </DialogClose>
          <Button type="submit" :disabled="!canSubmit">
            {{ busy ? 'Joining…' : 'Join Pipeline' }}
          </Button>
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
  color: var(--text-dim);
  font-size: 0.75rem;
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
  font-size: 0.75rem;
  color: var(--text-dim);
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
  font-size: 0.75rem;
  color: var(--color-status-abandoned);
}

.dialog-actions {
  padding-top: 0.5rem;
}
</style>
