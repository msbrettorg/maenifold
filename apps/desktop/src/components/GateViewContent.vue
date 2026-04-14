<script setup lang="ts">
import GateTaskDetail from './GateTaskDetail.vue';
import GateTaskTable from './GateTaskTable.vue';
import type { GateTask, GateTaskHistoryEntry } from '../lib/gate';

type SortColumn = 'id' | 'name' | 'process' | 'progress';
type SortDirection = 'asc' | 'desc';

defineProps<{
  tasks: GateTask[];
  selectedTaskId: string;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
  activeTask: GateTask | null;
  body: string;
  history: GateTaskHistoryEntry[];
  bodyLoading: boolean;
  historyLoading: boolean;
  bodyError: string;
  historyError: string;
}>();

defineEmits<{
  sort: [column: SortColumn];
  select: [task: GateTask];
  close: [];
}>();
</script>

<template>
  <div class="gate-content">
    <GateTaskTable
      :tasks="tasks"
      :selected-task-id="selectedTaskId"
      :sort-column="sortColumn"
      :sort-direction="sortDirection"
      @sort="$emit('sort', $event)"
      @select="$emit('select', $event)"
    />
    <GateTaskDetail
      :task="activeTask"
      :body="body"
      :history="history"
      :body-loading="bodyLoading"
      :history-loading="historyLoading"
      :body-error="bodyError"
      :history-error="historyError"
      @close="$emit('close')"
    />
  </div>
</template>

<style scoped>
.gate-content {
  display: flex;
  gap: 1rem;
  min-height: 0;
}

@media (max-width: 1100px) {
  .gate-content {
    flex-direction: column;
  }
}
</style>
