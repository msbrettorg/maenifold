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
</script>

<template>
  <div class="gate-table-wrap panel">
    <table class="gate-table">
      <thead>
        <tr>
          <th class="gate-th--sortable" @click="emit('sort', 'id')">ID<span class="gate-sort-icon">{{ sortIndicator('id') }}</span></th>
          <th class="gate-th--sortable" @click="emit('sort', 'name')">Name<span class="gate-sort-icon">{{ sortIndicator('name') }}</span></th>
          <th class="gate-th--sortable" @click="emit('sort', 'process')">Process<span class="gate-sort-icon">{{ sortIndicator('process') }}</span></th>
          <th class="gate-th--sortable" @click="emit('sort', 'progress')">Progress<span class="gate-sort-icon">{{ sortIndicator('progress') }}</span></th>
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
  border-bottom: 1px solid var(--border-default);
}

.gate-table th {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--bg-panel);
}

.gate-th--sortable,
.gate-row--active,
.gate-table tbody tr {
  cursor: pointer;
}

.gate-th--sortable {
  user-select: none;
}

.gate-th--sortable:hover {
  color: var(--text-primary);
}

.gate-sort-icon,
.gate-subtitle,
.gate-empty {
  color: var(--text-muted);
}

.gate-sort-icon,
.gate-id,
.gate-subtitle,
.gate-issue {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-row--active {
  background: color-mix(in srgb, var(--border-focus) 12%, var(--bg-panel));
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
