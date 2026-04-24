<script setup lang="ts">
import { computed, h } from 'vue';
import {
  type ColumnDef,
  type SortingState,
  FlexRender,
  getCoreRowModel,
  useVueTable,
} from '@tanstack/vue-table';
import { ArrowDown, ArrowUp, ArrowUpDown, TriangleAlert } from 'lucide-vue-next';

import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { ScrollArea } from '@/components/ui/scroll-area';
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

function sortAriaOrder(column: SortColumn): 'ascending' | 'descending' | 'none' {
  if (props.sortColumn !== column) return 'none';
  return props.sortDirection === 'asc' ? 'ascending' : 'descending';
}

function SortIcon(column: SortColumn) {
  if (props.sortColumn !== column) {
    return h(ArrowUpDown, { class: 'h-3.5 w-3.5 opacity-50' });
  }
  return props.sortDirection === 'asc'
    ? h(ArrowUp, { class: 'h-3.5 w-3.5' })
    : h(ArrowDown, { class: 'h-3.5 w-3.5' });
}

function SortableHeader(label: string, column: SortColumn) {
  return h(
    Button,
    {
      variant: 'ghost',
      size: 'sm',
      class: '-ml-3 h-8 data-[state=open]:bg-accent',
      onClick: () => emit('sort', column),
    },
    () => [h('span', label), SortIcon(column)],
  );
}

const columns: ColumnDef<GateTask>[] = [
  {
    id: 'id',
    accessorKey: 'taskId',
    header: () => SortableHeader('Task ID', 'id'),
  },
  {
    id: 'name',
    accessorKey: 'title',
    header: () => SortableHeader('Task', 'name'),
  },
  {
    id: 'process',
    accessorFn: (task) => getTaskPipelineLabel(task),
    header: () => SortableHeader('Process', 'process'),
  },
  {
    id: 'progress',
    accessorKey: 'state',
    header: () => SortableHeader('Progress', 'progress'),
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
</script>

<template>
  <div class="flex-[2] min-h-96 rounded-lg border bg-background">
    <ScrollArea class="h-full">
    <Table class="w-full table-fixed">
      <colgroup>
        <col class="w-[100px]" />
        <col />
        <col class="w-20" />
        <col class="w-[200px]" />
      </colgroup>
      <TableHeader class="bg-muted/40">
        <TableRow
          v-for="headerGroup in table.getHeaderGroups()"
          :key="headerGroup.id"
        >
          <TableHead
            v-for="header in headerGroup.headers"
            :key="header.id"
            :aria-sort="sortAriaOrder(header.column.id as SortColumn)"
            class="sticky top-0 z-10 bg-muted/40 text-text-primary font-medium"
          >
            <FlexRender
              v-if="!header.isPlaceholder"
              :render="header.column.columnDef.header"
              :props="header.getContext()"
            />
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow
          v-for="row in table.getRowModel().rows"
          :key="row.id"
          :data-state="row.original.taskId === props.selectedTaskId ? 'selected' : undefined"
          :title="rowTitle(row.original)"
          class="cursor-pointer hover:bg-surface-high data-[state=selected]:bg-surface-high data-[state=selected]:text-text-primary"
          @click="emit('select', row.original)"
        >
          <TableCell class="truncate font-mono text-xs text-text-secondary">
            {{ row.original.taskId }}
          </TableCell>
          <TableCell class="truncate text-text-primary">
            <div class="flex min-w-0 items-center gap-2">
              <span :title="row.original.title" class="min-w-0 flex-1 truncate text-text-primary">
                {{ row.original.title }}
              </span>
              <span
                v-if="row.original.issue"
                aria-hidden="true"
                class="h-1.5 w-1.5 shrink-0 rounded-full bg-destructive"
              />
            </div>
          </TableCell>
          <TableCell class="truncate text-text-secondary">
            <span>{{ processLabel(row.original) }}</span>
            <TriangleAlert
              v-if="isTaskPipelineDigestStale(row.original)"
              role="img"
              aria-label="Pipeline definition has changed since this task was created"
              :title="pipelineStaleTitle(row.original)"
              class="ml-1.5 inline h-3.5 w-3.5 cursor-help text-warning"
            />
          </TableCell>
          <TableCell>
            <div class="flex flex-wrap gap-1">
              <GateTaskProgress :task="row.original" />
            </div>
          </TableCell>
        </TableRow>
        <TableRow v-if="!props.tasks.length" class="hover:bg-transparent">
          <TableCell colspan="4" class="text-center text-text-secondary">
            No gate tasks match the current filters.
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
    </ScrollArea>
  </div>
</template>
