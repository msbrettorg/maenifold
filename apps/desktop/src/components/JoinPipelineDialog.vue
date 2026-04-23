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
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
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
    <DialogContent class="w-[min(52rem,100%)] max-h-[min(90vh,52rem)] overflow-auto gap-0 p-0 sm:max-w-[52rem]" :show-close-button="false">
      <DialogHeader class="flex flex-row items-start justify-between gap-4 p-6 pb-0">
        <div class="grid gap-[0.35rem] text-left">
          <DialogTitle>Join pipeline</DialogTitle>
          <DialogDescription>Merge another pipeline session into this one by mapping source nodes to target nodes.</DialogDescription>
        </div>
        <DialogClose as-child>
          <Button type="button" variant="ghost" :disabled="busy">Close</Button>
        </DialogClose>
      </DialogHeader>

      <form class="grid gap-4 px-6 pb-6" @submit.prevent="handleSubmit">
        <div class="grid gap-[0.35rem] min-w-0 text-[var(--text-secondary)] text-xs">
          <Label for="join-source-session-id">Source pipeline session ID</Label>
          <Input id="join-source-session-id" v-model="sourceSessionId" type="text" placeholder="session-abc123" />
        </div>

        <div class="grid gap-3">
          <div class="flex items-center justify-between gap-2">
            <h4 class="m-0 text-xs text-[var(--text-secondary)]">Edge mappings</h4>
            <Button type="button" variant="ghost" :disabled="busy" @click="addEdgeRow">Add Edge</Button>
          </div>

          <div v-for="(edge, index) in edgeRows" :key="`join-edge-${index}`" class="grid grid-cols-[repeat(auto-fit,minmax(10rem,1fr))_auto] gap-3 items-end">
            <div class="grid gap-[0.35rem] min-w-0 text-[var(--text-secondary)] text-xs">
              <Label :for="`join-edge-source-${index}`">Source node ID</Label>
              <Input :id="`join-edge-source-${index}`" v-model="edge.source" type="text" placeholder="source-node-id" />
            </div>
            <div class="grid gap-[0.35rem] min-w-0 text-[var(--text-secondary)] text-xs">
              <Label :for="`join-edge-target-${index}`">Target node ID</Label>
              <Input :id="`join-edge-target-${index}`" v-model="edge.target" type="text" placeholder="target-node-id" />
            </div>
            <div class="grid gap-[0.35rem] min-w-0 text-[var(--text-secondary)] text-xs">
              <Label :for="`join-edge-type-${index}`">Edge type</Label>
              <Select v-model="edge.type">
                <SelectTrigger :id="`join-edge-type-${index}`" class="w-full">
                  <SelectValue placeholder="Select edge type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="success">Success</SelectItem>
                  <SelectItem value="always">Always</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Button type="button" variant="ghost" class="justify-self-end" :disabled="busy || edgeRows.length <= 1" @click="removeEdgeRow(index)">
              Remove
            </Button>
          </div>
        </div>

        <p v-if="formError" class="m-0 text-xs text-[var(--color-status-abandoned)]" role="alert">{{ formError }}</p>

        <DialogFooter class="pt-2">
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
