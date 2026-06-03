<script setup lang="ts">
// Shared surgery edge editor (T-600.5). A controlled list of linking edges used
// by BOTH the graft and join dialogs. Each row builds an `EdgeDraft` with valid
// source/target (picked from the supplied node options), a required edge `type`,
// and — only for `conditional` — the `path` + `equals` fields the backend
// validator demands. Per-row validity is surfaced inline; the parent dialog
// gates submit on `buildEdges`.
import {
  SURGERY_EDGE_TYPES,
  blankEdgeDraft,
  edgeDraftError,
  type EdgeDraft,
} from '../../../lib/surgeryEdges';
import { Plus, X } from '@lucide/vue';

export interface NodeOption {
  id: string;
  label: string;
}

const props = defineProps<{
  modelValue: EdgeDraft[];
  nodeOptions: NodeOption[];
}>();

const emit = defineEmits<{
  'update:modelValue': [value: EdgeDraft[]];
}>();

function commit(next: EdgeDraft[]): void {
  emit('update:modelValue', next);
}

function addRow(): void {
  commit([...props.modelValue, blankEdgeDraft()]);
}

function removeRow(index: number): void {
  commit(props.modelValue.filter((_, i) => i !== index));
}

function patchRow(index: number, patch: Partial<EdgeDraft>): void {
  commit(props.modelValue.map((row, i) => (i === index ? { ...row, ...patch } : row)));
}

function onSource(index: number, event: Event): void {
  patchRow(index, { source: (event.target as HTMLSelectElement).value });
}

function onTarget(index: number, event: Event): void {
  patchRow(index, { target: (event.target as HTMLSelectElement).value });
}

function onType(index: number, event: Event): void {
  const type = (event.target as HTMLSelectElement).value as EdgeDraft['type'];
  patchRow(index, { type });
}

function onPath(index: number, event: Event): void {
  patchRow(index, { path: (event.target as HTMLInputElement).value });
}

function onEquals(index: number, event: Event): void {
  patchRow(index, { equals: (event.target as HTMLInputElement).value });
}

function rowError(row: EdgeDraft): string | null {
  return edgeDraftError(row);
}
</script>

<template>
  <div class="edge-editor">
    <div v-if="modelValue.length === 0" class="edge-empty">No linking edges.</div>
    <div v-for="(row, index) in modelValue" :key="index" class="edge-row">
      <div class="edge-line">
        <select class="edge-input" :value="row.source" @change="onSource(index, $event)">
          <option value="">source…</option>
          <option v-for="opt in nodeOptions" :key="opt.id" :value="opt.id">{{ opt.label }}</option>
        </select>
        <span class="edge-arrow" aria-hidden="true">→</span>
        <select class="edge-input" :value="row.target" @change="onTarget(index, $event)">
          <option value="">target…</option>
          <option v-for="opt in nodeOptions" :key="opt.id" :value="opt.id">{{ opt.label }}</option>
        </select>
        <select class="edge-input edge-type" :value="row.type" @change="onType(index, $event)">
          <option v-for="type in SURGERY_EDGE_TYPES" :key="type" :value="type">{{ type }}</option>
        </select>
        <button
          type="button"
          class="edge-remove"
          aria-label="Remove edge"
          title="Remove edge"
          @click="removeRow(index)"
        >
          <X :size="13" :stroke-width="2" />
        </button>
      </div>
      <div v-if="row.type === 'conditional'" class="edge-cond">
        <input
          class="edge-input"
          type="text"
          placeholder="path (e.g. output.decision)"
          :value="row.path ?? ''"
          @input="onPath(index, $event)"
        />
        <input
          class="edge-input"
          type="text"
          placeholder="equals"
          :value="row.equals ?? ''"
          @input="onEquals(index, $event)"
        />
      </div>
      <span v-if="rowError(row)" class="edge-err">{{ rowError(row) }}</span>
    </div>

    <button type="button" class="edge-add" @click="addRow">
      <Plus :size="13" :stroke-width="2" /> Add edge
    </button>
  </div>
</template>

<style scoped>
.edge-editor {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.edge-empty {
  font-size: 0.74rem;
  color: var(--text-muted);
}

.edge-row {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.edge-line {
  display: flex;
  align-items: center;
  gap: 0.3rem;
}

.edge-cond {
  display: flex;
  gap: 0.3rem;
  padding-left: 0.2rem;
}

.edge-input {
  flex: 1;
  min-width: 0;
  padding: 0.3rem 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  font: inherit;
  font-size: 0.78rem;
  outline: none;
}

.edge-type {
  flex: 0 0 7rem;
}

.edge-arrow {
  flex-shrink: 0;
  color: var(--text-muted);
}

.edge-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0.25rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
}

.edge-remove:hover {
  color: var(--bg-danger, #d9534f);
  background: var(--bg-hover);
}

.edge-err {
  font-size: 0.7rem;
  color: var(--bg-danger, #d9534f);
}

.edge-add {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  align-self: flex-start;
  padding: 0.3rem 0.55rem;
  border: 1px dashed var(--border-color);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
}

.edge-add:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
</style>
