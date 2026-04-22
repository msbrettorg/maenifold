<script setup lang="ts">
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import GateTaskProgress from './GateTaskProgress.vue';
import type { GateTask } from '../lib/gate';
import { getTaskPathHint, getTaskPipelineLabel, isTaskPipelineDigestStale } from '../lib/gate-display';

type SortColumn = 'id' | 'name' | 'process' | 'progress';
type SortDirection = 'asc' | 'desc';
type ColumnKey = 'id' | 'name' | 'process' | 'progress';

const props = defineProps<{
  tasks: GateTask[];
  selectedTaskId: string;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
}>();

const emit = defineEmits<{
  select: [task: GateTask];
  sort: [column: SortColumn];
}>();

// Resizable column widths (px). `name` column flexes to fill remaining space.
const MIN_WIDTHS: Record<ColumnKey, number> = {
  id: 70,
  name: 120,
  process: 80,
  progress: 100,
};

const columnWidths = ref<Record<ColumnKey, number>>({
  id: 100,
  name: 240,
  process: 120,
  progress: 200,
});

let resizeState: {
  column: ColumnKey;
  startX: number;
  startWidth: number;
} | null = null;

function onResizeStart(column: ColumnKey, event: PointerEvent) {
  event.preventDefault();
  event.stopPropagation();
  resizeState = {
    column,
    startX: event.clientX,
    startWidth: columnWidths.value[column],
  };
  window.addEventListener('pointermove', onResizeMove);
  window.addEventListener('pointerup', onResizeEnd);
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function onResizeMove(event: PointerEvent) {
  if (!resizeState) return;
  const delta = event.clientX - resizeState.startX;
  const min = MIN_WIDTHS[resizeState.column];
  const next = Math.max(min, resizeState.startWidth + delta);
  columnWidths.value[resizeState.column] = next;
}

function onResizeEnd() {
  resizeState = null;
  window.removeEventListener('pointermove', onResizeMove);
  window.removeEventListener('pointerup', onResizeEnd);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

function processValue(task: GateTask) {
  return getTaskPipelineLabel(task);
}

function processLabel(task: GateTask) {
  const pipeline = processValue(task);
  return task.location === 'done' ? `${pipeline} done` : pipeline;
}

function isPipelineStale(task: GateTask) {
  return isTaskPipelineDigestStale(task);
}

function pipelineStaleTitle(task: GateTask) {
  return task.issue?.trim() || 'Pipeline definition has changed since this task was created.';
}

function sortIndicator(column: SortColumn) {
  if (props.sortColumn !== column) return '';
  return props.sortDirection === 'asc' ? ' ▲' : ' ▼';
}

function sortOrder(column: SortColumn) {
  if (props.sortColumn !== column) return 'none';
  return props.sortDirection === 'asc' ? 'ascending' : 'descending';
}

function rowTitle(task: GateTask) {
  return [
    `${task.taskId} — ${task.title}`,
    `File: ${getTaskPathHint(task.path)}`,
    task.issue ? `Issue: ${task.issue}` : '',
  ].filter(Boolean).join('\n');
}
</script>

<template>
  <div class="gate-table-wrap">
    <table class="gate-table" role="table" aria-label="Gate task list">
      <colgroup>
        <col :style="{ width: columnWidths.id + 'px' }" />
        <col :style="{ width: columnWidths.name + 'px' }" />
        <col :style="{ width: columnWidths.process + 'px' }" />
        <col :style="{ width: columnWidths.progress + 'px' }" />
      </colgroup>
      <thead>
        <tr>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('id')">
            <Button variant="ghost" class="gate-th-button" @click="emit('sort', 'id')">
              <span>Task ID</span>
              <span class="gate-sort-icon">{{ sortIndicator('id') }}</span>
            </Button>
            <span
              class="gate-col-resizer"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize Task ID column"
              @pointerdown="onResizeStart('id', $event)"
            ></span>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('name')">
            <Button variant="ghost" class="gate-th-button" @click="emit('sort', 'name')">
              <span>Task</span>
              <span class="gate-sort-icon">{{ sortIndicator('name') }}</span>
            </Button>
            <span
              class="gate-col-resizer"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize Task column"
              @pointerdown="onResizeStart('name', $event)"
            ></span>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('process')">
            <Button variant="ghost" class="gate-th-button" @click="emit('sort', 'process')">
              <span>Process</span>
              <span class="gate-sort-icon">{{ sortIndicator('process') }}</span>
            </Button>
            <span
              class="gate-col-resizer"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize Process column"
              @pointerdown="onResizeStart('process', $event)"
            ></span>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('progress')">
            <Button variant="ghost" class="gate-th-button" @click="emit('sort', 'progress')">
              <span>Progress</span>
              <span class="gate-sort-icon">{{ sortIndicator('progress') }}</span>
            </Button>
            <span
              class="gate-col-resizer"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize Progress column"
              @pointerdown="onResizeStart('progress', $event)"
            ></span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="task in props.tasks"
          :key="task.path"
          :class="[
            'hover:bg-bg-surface-hover transition-colors cursor-pointer',
            {
              'bg-brand/10 border-l-2 border-brand [&>td]:bg-brand/10 [&>td:first-child]:border-l-2 [&>td:first-child]:border-brand': task.taskId === props.selectedTaskId,
              'gate-row--issue': task.issue,
            },
          ]"
          :title="rowTitle(task)"
          @click="emit('select', task)"
        >
          <td class="gate-id">{{ task.taskId }}</td>
          <td class="gate-task">
            <div class="gate-task__title">
              <span class="gate-title line-clamp-2">{{ task.title }}</span>
              <span v-if="task.issue" class="gate-issue-dot" aria-hidden="true"></span>
            </div>
          </td>
          <td>
            <span class="gate-process">{{ processLabel(task) }}</span>
            <span
              v-if="isPipelineStale(task)"
              class="gate-process__warning"
              role="img"
              aria-label="Pipeline definition has changed since this task was created"
              :title="pipelineStaleTitle(task)"
            >⚠</span>
          </td>
          <td><GateTaskProgress :task="task" /></td>
        </tr>
      </tbody>
    </table>
    <p v-if="!props.tasks.length" class="gate-empty">No gate tasks match the current filters.</p>
  </div>
</template>

<style scoped>
.gate-table-wrap {
  flex: 2;
  min-height: 24rem;
  padding: 0.5rem;
  overflow: auto;
  background: var(--color-bg-app);
  border: 1px solid var(--color-border-card);
  border-radius: 8px;
  scrollbar-width: thin;
  scrollbar-color: var(--color-border-card) transparent;
}

.gate-table-wrap::-webkit-scrollbar {
  width: 8px;
}

.gate-table-wrap::-webkit-scrollbar-track {
  background: transparent;
}

.gate-table-wrap::-webkit-scrollbar-thumb {
  background: var(--color-border-subtle);
  border-radius: 4px;
}

.gate-table-wrap::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-secondary);
}

.gate-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0 2px;
  table-layout: fixed;
}

.gate-table th,
.gate-table td {
  padding: 0.85rem 1rem;
  text-align: left;
  vertical-align: middle;
}

.gate-table th {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--surface);
}

.gate-col-resizer {
  position: absolute;
  top: 0;
  right: 0;
  width: 6px;
  height: 100%;
  cursor: col-resize;
  user-select: none;
  touch-action: none;
  z-index: 2;
  background: transparent;
  transition: background 120ms ease;
}

.gate-col-resizer:hover,
.gate-col-resizer:active {
  background: var(--color-border-subtle);
}

.gate-table tbody tr td {
  background: var(--surface);
  transition: background 160ms ease;
}

.gate-table tbody td:first-child,
.gate-table tbody td:last-child {
  position: relative;
}

.gate-table tbody td:first-child {
  border-radius: 0.25rem 0 0 0.25rem;
  padding-left: 1.25rem;
}

.gate-table tbody td:last-child {
  border-radius: 0 0.25rem 0.25rem 0;
}

.gate-th--sortable {
  padding: 0;
  position: relative;
}

.gate-th-button {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  width: 100%;
  padding: 0.85rem 1rem;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  user-select: none;
  cursor: pointer;
}

.gate-th-button:hover {
  color: var(--color-text-primary);
}

.gate-sort-icon,
.gate-empty {
  color: var(--color-text-secondary);
}

.gate-sort-icon,
.gate-id,
.gate-process {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-row--issue .gate-title {
  color: var(--color-text-primary);
}

.gate-title {
  font-weight: 400;
}

.gate-task__title {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
}

.gate-title {
  min-width: 0;
  overflow-wrap: anywhere;
}

.gate-issue-dot {
  flex-shrink: 0;
}

.gate-process {
  color: var(--color-text-secondary);
}

.gate-process__warning {
  margin-left: 0.35rem;
  color: var(--amber, var(--terracotta));
  font-size: var(--text-tiny);
  cursor: help;
}

.gate-issue-dot {
  width: 0.4rem;
  height: 0.4rem;
  border-radius: 4px;
  background: var(--terracotta);
}

.gate-empty {
  padding: 1rem;
}
</style>
