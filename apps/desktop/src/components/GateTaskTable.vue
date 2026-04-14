<script setup lang="ts">
import GateTaskProgress from './GateTaskProgress.vue';
import type { GateTask } from '../lib/gate';

type SortColumn = 'id' | 'name' | 'process' | 'progress';
type SortDirection = 'asc' | 'desc';

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

function processValue(task: GateTask) {
  return task.pipeline?.trim() || 'unbound';
}

function processLabel(task: GateTask) {
  const pipeline = processValue(task).toUpperCase();
  return task.location === 'done' ? `${pipeline} done` : pipeline;
}

function sortIndicator(column: SortColumn) {
  if (props.sortColumn !== column) return '';
  return props.sortDirection === 'asc' ? ' ▲' : ' ▼';
}

function sortOrder(column: SortColumn) {
  if (props.sortColumn !== column) return 'none';
  return props.sortDirection === 'asc' ? 'ascending' : 'descending';
}
</script>

<template>
  <div class="gate-table-wrap">
    <table class="gate-table" role="table" aria-label="Gate task list">
      <thead>
        <tr>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('id')">
            <button type="button" class="gate-th-button" @click="emit('sort', 'id')">
              <span>ID</span>
              <span class="gate-sort-icon">{{ sortIndicator('id') }}</span>
            </button>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('name')">
            <button type="button" class="gate-th-button" @click="emit('sort', 'name')">
              <span>Name</span>
              <span class="gate-sort-icon">{{ sortIndicator('name') }}</span>
            </button>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('process')">
            <button type="button" class="gate-th-button" @click="emit('sort', 'process')">
              <span>Process</span>
              <span class="gate-sort-icon">{{ sortIndicator('process') }}</span>
            </button>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('progress')">
            <button type="button" class="gate-th-button" @click="emit('sort', 'progress')">
              <span>Progress</span>
              <span class="gate-sort-icon">{{ sortIndicator('progress') }}</span>
            </button>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="task in props.tasks"
          :key="task.path"
          :class="{ 'gate-row--active': task.taskId === props.selectedTaskId, 'gate-row--issue': task.issue }"
          @click="emit('select', task)"
        >
          <td class="gate-id">{{ task.taskId }}</td>
          <td>
            <div class="gate-title">{{ task.title }}</div>
            <div class="gate-subtitle">{{ task.path }}</div>
            <div v-if="task.issue" class="gate-issue">{{ task.issue }}</div>
          </td>
          <td>{{ processLabel(task) }}</td>
          <td><GateTaskProgress :task="task" /></td>
        </tr>
      </tbody>
    </table>
    <p v-if="!props.tasks.length" class="gate-empty">No gate tasks match the current filters.</p>
  </div>
</template>

<style scoped>
.gate-table-wrap {
  flex: 1.4;
  min-height: 24rem;
  padding: 0;
  overflow: auto;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  scrollbar-width: thin;
  scrollbar-color: var(--border) transparent;
}

.gate-table-wrap::-webkit-scrollbar {
  width: 8px;
}

.gate-table-wrap::-webkit-scrollbar-track {
  background: transparent;
}

.gate-table-wrap::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}

.gate-table-wrap::-webkit-scrollbar-thumb:hover {
  background: var(--muted-foreground);
}

.gate-table {
  width: 100%;
  border-collapse: collapse;
}

.gate-table th,
.gate-table td {
  padding: 0.85rem 1rem;
  text-align: left;
  vertical-align: top;
  border-bottom: 1px solid var(--border);
}

.gate-table th {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--card);
}

.gate-row--active,
.gate-table tbody tr {
  cursor: pointer;
}

.gate-th--sortable {
  padding: 0;
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
  color: var(--foreground);
}

.gate-sort-icon,
.gate-subtitle,
.gate-empty {
  color: var(--muted-foreground);
}

.gate-sort-icon,
.gate-id,
.gate-subtitle,
.gate-issue {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-row--active {
  background: color-mix(in srgb, var(--ring) 12%, var(--card));
}

.gate-row--issue .gate-title,
.gate-issue {
  color: var(--color-status-abandoned);
}

.gate-title {
  font-weight: var(--weight-title);
}

.gate-issue {
  margin-top: 0.35rem;
  white-space: pre-wrap;
}

.gate-empty {
  padding: 1rem;
}
</style>
