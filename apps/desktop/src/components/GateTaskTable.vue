<script setup lang="ts">
import GateTaskProgress from './GateTaskProgress.vue';
import type { GateTask } from '../lib/gate';
import { getTaskPathHint } from '../lib/gate-display';

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
  const pipeline = processValue(task);
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
      <thead>
        <tr>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('id')">
            <button type="button" class="gate-th-button" @click="emit('sort', 'id')">
              <span>Task ID</span>
              <span class="gate-sort-icon">{{ sortIndicator('id') }}</span>
            </button>
          </th>
          <th role="columnheader" scope="col" class="gate-th--sortable" :aria-sort="sortOrder('name')">
            <button type="button" class="gate-th-button" @click="emit('sort', 'name')">
              <span>Task</span>
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
          :title="rowTitle(task)"
          @click="emit('select', task)"
        >
          <td class="gate-id">{{ task.taskId }}</td>
          <td class="gate-task">
            <div class="gate-task__title">
              <span class="gate-title">{{ task.title }}</span>
              <span v-if="task.issue" class="gate-issue-dot" aria-hidden="true"></span>
            </div>
          </td>
          <td><span class="gate-process">{{ processLabel(task) }}</span></td>
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
  padding: 0.5rem;
  overflow: auto;
  background: #131313;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  scrollbar-width: thin;
  scrollbar-color: rgba(66, 71, 83, 0.4) transparent;
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
  background: #1c1b1b;
}

.gate-table tbody tr {
  cursor: pointer;
}

.gate-table tbody tr:nth-child(odd) td {
  background: #1c1b1b;
  transition: background 160ms ease;
}

.gate-table tbody tr:nth-child(even) td {
  background: #201f1f;
  transition: background 160ms ease;
}

.gate-table tbody tr:hover td {
  background: #2a2a2a;
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
.gate-empty {
  color: var(--muted-foreground);
}

.gate-sort-icon,
.gate-id,
.gate-process {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-row--active {
  background: transparent;
}

.gate-row--active td {
  background: #2a2a2a;
}

.gate-row--active td:first-child::before {
  content: '';
  position: absolute;
  top: 0.45rem;
  bottom: 0.45rem;
  left: 0;
  width: 4px;
  border-radius: 4px;
  background: #4d8eff;
}

.gate-row--issue .gate-title {
  color: var(--foreground);
}

.gate-title {
  font-weight: 400;
}

.gate-task__title {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.gate-process {
  color: var(--muted-foreground);
  text-transform: lowercase;
}

.gate-issue-dot {
  width: 0.4rem;
  height: 0.4rem;
  border-radius: 4px;
  background: #ffb4ab;
}

.gate-empty {
  padding: 1rem;
}
</style>
