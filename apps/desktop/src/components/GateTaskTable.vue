<script setup lang="ts">
import { computed, h } from 'vue';
import {
  type ColumnDef,
  type SortingState,
  FlexRender,
  getCoreRowModel,
  useVueTable,
} from '@tanstack/vue-table';
import {
  ChevronDown,
  ChevronsUpDown,
  ChevronUp,
  Circle,
  TriangleAlert,
} from 'lucide-vue-next';

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
import type { GateTask } from '../lib/gate';
import { gateTaskSelectionKey } from '../composables/useGateTaskDetail';
import {
  formatStageLabel,
  gateStageFamily,
  getTaskPathHint,
  getTaskPipelineLabel,
  isTaskPipelineDigestStale,
  isTaskStale,
} from '../lib/gate-display';

type SortColumn = 'id' | 'name' | 'process' | 'progress' | 'lastUpdated' | 'priority' | 'default';
type SortDirection = 'asc' | 'desc';

const props = defineProps<{
  tasks: GateTask[];
  selectedTaskKey: string;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
}>();

const emit = defineEmits<{
  select: [task: GateTask];
  sort: [column: SortColumn];
}>();

function processLabel(task: GateTask) {
  return getTaskPipelineLabel(task);
}

const STAGE_CHIP_CLASS: Record<string, string> = {
  pending: 'bg-status-pending/15 text-status-pending',
  swe: 'bg-status-active/15 text-status-active',
  'red-team': 'bg-status-stuck/15 text-status-stuck',
  'blue-team': 'bg-status-ready/15 text-status-ready',
  done: 'bg-status-completed/15 text-status-completed',
  failed: 'bg-status-error/15 text-status-error',
  neutral: 'bg-status-pending/15 text-status-pending',
};

function stageChipClass(task: GateTask) {
  const family = gateStageFamily(task.state ?? '');
  return STAGE_CHIP_CLASS[family] ?? STAGE_CHIP_CLASS.pending;
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

function formatLastUpdated(value?: string | null) {
  if (!value) return '—';
  const date = new Date(value);
  const time = date.getTime();
  if (Number.isNaN(time)) return '—';
  const minutes = Math.floor((Date.now() - time) / 60000);
  if (minutes < 0) return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return 'yesterday';
  if (days < 7) return `${days}d ago`;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function sortAriaOrder(column: SortColumn): 'ascending' | 'descending' | 'none' {
  if (props.sortColumn !== column) return 'none';
  return props.sortDirection === 'asc' ? 'ascending' : 'descending';
}

function SortIcon(column: SortColumn) {
  if (props.sortColumn !== column) {
    return h(ChevronsUpDown, { class: 'h-3.5 w-3.5 opacity-50' });
  }
  return props.sortDirection === 'asc'
    ? h(ChevronUp, { class: 'h-3.5 w-3.5' })
    : h(ChevronDown, { class: 'h-3.5 w-3.5' });
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

const PRIORITY_META: Record<string, { colorClass: string; filled: boolean }> = {
  P0: { colorClass: 'text-status-error', filled: true },
  P1: { colorClass: 'text-status-stuck', filled: true },
  P2: { colorClass: 'text-status-pending', filled: true },
  P3: { colorClass: 'text-muted-foreground', filled: true },
  P4: { colorClass: 'text-muted-foreground', filled: false },
};

function priorityMeta(p: string | null | undefined) {
  return p ? (PRIORITY_META[p] ?? null) : null;
}

function getColumnWidthStyle(columnId: SortColumn) {
  const column = table.getColumn(columnId);
  return column ? { width: `${column.getSize()}px` } : undefined;
}

const columns: ColumnDef<GateTask>[] = [
  { id: 'priority', accessorKey: 'priority', header: () => SortableHeader('Priority', 'priority'), size: 64, minSize: 56, maxSize: 96 },
  { id: 'id', accessorKey: 'taskId', header: () => SortableHeader('Task ID', 'id'), size: 120, minSize: 100, maxSize: 180 },
  { id: 'name', accessorKey: 'title', header: () => SortableHeader('Task', 'name'), size: 320, minSize: 180, maxSize: 9999 },
  { id: 'process', accessorFn: (task) => getTaskPipelineLabel(task), header: () => SortableHeader('Process', 'process'), size: 140, minSize: 120, maxSize: 220 },
  { id: 'progress', accessorKey: 'state', header: () => SortableHeader('Stage', 'progress'), size: 160, minSize: 140, maxSize: 260 },
  { id: 'lastUpdated', accessorKey: 'lastUpdatedAt', header: () => SortableHeader('Last Updated', 'lastUpdated'), size: 140, minSize: 120, maxSize: 220 },
];

const sortingState = computed<SortingState>(() => [
  { id: props.sortColumn, desc: props.sortDirection === 'desc' },
]);

const table = useVueTable<GateTask>({
  get data() {
    return props.tasks;
  },
  columns,
  enableColumnResizing: true,
  columnResizeMode: 'onChange',
  state: {
    get sorting() {
      return sortingState.value;
    },
  },
  manualSorting: true,
  getRowId: gateTaskSelectionKey,
  getCoreRowModel: getCoreRowModel(),
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col rounded-lg border bg-background">
    <ScrollArea class="h-full">
      <Table class="w-full table-fixed">
        <TableHeader class="sticky top-0 z-10 bg-surface">
          <TableRow
            v-for="headerGroup in table.getHeaderGroups()"
            :key="headerGroup.id"
          >
            <TableHead
              v-for="header in headerGroup.headers"
              :key="header.id"
              :aria-sort="sortAriaOrder(header.column.id as SortColumn)"
              :style="{ width: `${header.getSize()}px` }"
              class="group relative bg-surface text-text-primary font-medium"
            >
              <FlexRender
                v-if="!header.isPlaceholder"
                :render="header.column.columnDef.header"
                :props="header.getContext()"
              />
              <div
                v-if="!header.isPlaceholder"
                class="absolute right-0 top-0 h-full w-[3px] cursor-col-resize bg-border-subtle opacity-0 transition-opacity group-hover:opacity-100 hover:opacity-100 active:opacity-100 active:bg-brand"
                :class="header.column.getIsResizing() ? 'bg-brand opacity-100' : undefined"
                @mousedown.stop="header.getResizeHandler()($event)"
                @touchstart.stop="header.getResizeHandler()($event)"
              />
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow
            v-for="row in table.getRowModel().rows"
            :key="row.id"
            :data-task-id="row.original.taskId"
            :data-state="gateTaskSelectionKey(row.original) === props.selectedTaskKey ? 'selected' : undefined"
            :title="rowTitle(row.original)"
            class="cursor-pointer hover:bg-surface-high data-[state=selected]:bg-surface-high data-[state=selected]:text-text-primary"
            @click="emit('select', row.original)"
          >
            <TableCell :style="getColumnWidthStyle('priority')" class="w-16 text-xs">
              <span v-if="row.original.priority" class="inline-flex items-center gap-1.5" :class="priorityMeta(row.original.priority)?.colorClass">
                <Circle class="h-2.5 w-2.5 shrink-0" :class="priorityMeta(row.original.priority)?.filled ? 'fill-current' : ''" />
                <span class="font-mono font-medium">{{ row.original.priority }}</span>
              </span>
              <span v-else class="text-muted-foreground">—</span>
            </TableCell>
            <TableCell
              :style="getColumnWidthStyle('id')"
              class="truncate font-mono text-xs text-text-secondary"
            >
              {{ row.original.taskId }}
            </TableCell>
            <TableCell :style="getColumnWidthStyle('name')" class="truncate text-text-primary">
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
            <TableCell :style="getColumnWidthStyle('process')" class="truncate text-text-secondary">
              <span>{{ processLabel(row.original) }}</span>
              <TriangleAlert
                v-if="isTaskPipelineDigestStale(row.original)"
                role="img"
                aria-label="Pipeline definition has changed since this task was created"
                :title="pipelineStaleTitle(row.original)"
                class="ml-1.5 inline h-3.5 w-3.5 cursor-help text-warning"
              />
            </TableCell>
            <TableCell :style="getColumnWidthStyle('progress')">
              <span
                role="status"
                :aria-label="`Stage: ${formatStageLabel(row.original.state ?? '')}`"
                class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium"
                :class="stageChipClass(row.original)"
              >{{ formatStageLabel(row.original.state ?? '') }}</span>
            </TableCell>
            <TableCell
              :style="getColumnWidthStyle('lastUpdated')"
              class="text-xs text-text-secondary"
            >
              <div class="flex min-w-0 items-center gap-1.5">
                <time
                  v-if="row.original.lastUpdatedAt"
                  :datetime="row.original.lastUpdatedAt"
                  :title="row.original.lastUpdatedAt"
                  class="min-w-0 truncate"
                >{{ formatLastUpdated(row.original.lastUpdatedAt) }}</time>
                <span v-else class="truncate">—</span>
                <span
                  v-if="isTaskStale(row.original)"
                  aria-hidden="true"
                  class="h-2 w-2 shrink-0 rounded-full bg-status-stuck"
                  title="No activity in 24h"
                />
              </div>
            </TableCell>
          </TableRow>
          <TableRow v-if="!props.tasks.length" class="hover:bg-transparent">
            <TableCell colspan="6" class="text-center text-text-secondary">
              No gate tasks match the current filters.
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </ScrollArea>
  </div>
</template>
