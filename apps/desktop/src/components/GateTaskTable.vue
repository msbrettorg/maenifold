<script setup lang="ts">
import { computed } from 'vue';
import {
  type ColumnDef,
  type SortingState,
  getCoreRowModel,
  useVueTable,
} from '@tanstack/vue-table';

import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import GateTaskProgress from './GateTaskProgress.vue';
import type { GateTask } from '../lib/gate';
import {
  getTaskPathHint,
  getTaskPipelineLabel,
  isTaskPipelineDigestStale,
} from '../lib/gate-display';

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

function processLabel(task: GateTask) {
  const pipeline = getTaskPipelineLabel(task);
  return task.location === 'done' ? `${pipeline} done` : pipeline;
}

function pipelineStaleTitle(task: GateTask) {
  return task.issue?.trim() || 'Pipeline definition has changed since this task was created.';
}

function rowTitle(task: GateTask) {
  return [
    `${task.taskId} — ${task.title}`,
    `File: ${getTaskPathHint(task.path)}`,
    task.issue ? `Issue: ${task.issue}` : '',
  ]
    .filter(Boolean)
    .join('\n');
}

function sortIndicator(column: SortColumn) {
  if (props.sortColumn !== column) return '';
  return props.sortDirection === 'asc' ? ' ▲' : ' ▼';
}

function sortOrder(column: SortColumn): 'ascending' | 'descending' | 'none' {
  if (props.sortColumn !== column) return 'none';
  return props.sortDirection === 'asc' ? 'ascending' : 'descending';
}

const columns: ColumnDef<GateTask>[] = [
  {
    id: 'id',
    accessorKey: 'taskId',
    header: 'Task ID',
    meta: { label: 'Task ID' },
  },
  {
    id: 'name',
    accessorKey: 'title',
    header: 'Task',
    meta: { label: 'Task' },
  },
  {
    id: 'process',
    accessorFn: (task) => getTaskPipelineLabel(task),
    header: 'Process',
    meta: { label: 'Process' },
  },
  {
    id: 'progress',
    accessorKey: 'state',
    header: 'Progress',
    meta: { label: 'Progress' },
  },
];

const sortingState = computed<SortingState>(() => [
  { id: props.sortColumn, desc: props.sortDirection === 'desc' },
]);

const table = useVueTable<GateTask>({
  get data() {
    return props.tasks;
  },
  columns,
  state: {
    get sorting() {
      return sortingState.value;
    },
  },
  manualSorting: true,
  getRowId: (row) => row.taskId,
  getCoreRowModel: getCoreRowModel(),
});

function headerLabel(columnId: string) {
  const column = columns.find((c) => c.id === columnId);
  const meta = column?.meta as { label?: string } | undefined;
  return meta?.label ?? columnId;
}
</script>

<template>
  <div
    class="flex-[2] min-h-96 overflow-auto rounded-lg border border-[var(--color-border-card)] bg-[var(--color-bg-app)] p-2"
  >
    <Table class="border-separate" style="border-spacing: 0 2px">
      <TableHeader>
        <TableRow
          v-for="headerGroup in table.getHeaderGroups()"
          :key="headerGroup.id"
          class="border-b-0"
        >
          <TableHead
            v-for="header in headerGroup.headers"
            :key="header.id"
            :aria-sort="sortOrder(header.column.id as SortColumn)"
            class="sticky top-0 z-[1] bg-[var(--surface)] p-0"
          >
            <Button
              variant="ghost"
              class="flex h-auto w-full items-center justify-start gap-1.5 rounded-none px-4 py-3 text-left font-medium hover:bg-transparent hover:text-[var(--color-text-primary)]"
              @click="emit('sort', header.column.id as SortColumn)"
            >
              <span>{{ headerLabel(header.column.id) }}</span>
              <span class="font-mono text-[var(--text-tiny)] text-[var(--color-text-secondary)]">{{
                sortIndicator(header.column.id as SortColumn)
              }}</span>
            </Button>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow
          v-for="row in table.getRowModel().rows"
          :key="row.id"
          :class="[
            'cursor-pointer transition-colors hover:bg-[var(--bg-surface-hover)] [&>td]:bg-[var(--surface)] [&>td]:transition-colors',
            row.original.taskId === props.selectedTaskId
              ? 'border-l-2 border-brand bg-brand/10 [&>td]:bg-brand/10 [&>td:first-child]:border-l-2 [&>td:first-child]:border-brand'
              : '',
          ]"
          :title="rowTitle(row.original)"
          @click="emit('select', row.original)"
        >
          <TableCell class="rounded-l px-5 py-3.5 font-mono text-[var(--text-tiny)] text-[var(--color-text-secondary)]">
            {{ row.original.taskId }}
          </TableCell>
          <TableCell class="px-4 py-3.5">
            <div class="flex min-w-0 items-center gap-2">
              <span
                :class="[
                  'min-w-0 break-words font-normal line-clamp-2',
                  row.original.issue ? 'text-[var(--color-text-primary)]' : '',
                ]"
              >
                {{ row.original.title }}
              </span>
              <span
                v-if="row.original.issue"
                aria-hidden="true"
                class="h-1.5 w-1.5 shrink-0 rounded bg-[var(--terracotta)]"
              />
            </div>
          </TableCell>
          <TableCell class="px-4 py-3.5">
            <span class="font-mono text-[var(--text-tiny)] text-[var(--color-text-secondary)]">
              {{ processLabel(row.original) }}
            </span>
            <span
              v-if="isTaskPipelineDigestStale(row.original)"
              role="img"
              aria-label="Pipeline definition has changed since this task was created"
              :title="pipelineStaleTitle(row.original)"
              class="ml-1.5 cursor-help text-[var(--text-tiny)] text-[var(--amber,var(--terracotta))]"
            >
              ⚠
            </span>
          </TableCell>
          <TableCell class="rounded-r px-4 py-3.5">
            <GateTaskProgress :task="row.original" />
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
    <p
      v-if="!props.tasks.length"
      class="p-4 text-[var(--color-text-secondary)]"
    >
      No gate tasks match the current filters.
    </p>
  </div>
</template>
