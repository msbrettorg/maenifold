<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import type { PipelineListItem, PipelineRunSummary, PipelineStatus } from '../lib/pipeline-types';

const props = defineProps<{
  item: PipelineListItem;
  active: boolean;
}>();

const emit = defineEmits<{
  (e: 'select', pipelineId: string): void;
  (e: 'delete', item: PipelineListItem, event: Event): void;
}>();

const progressPct = computed(() =>
  props.item.nodeCount > 0
    ? (props.item.completedCount / props.item.nodeCount) * 100
    : 0,
);

const beadClass = computed(() => beadClassFor(props.item.status));
const statusTextClass = computed(() => statusTextClassFor(props.item.status));
const progressBarClass = computed(() => progressBarClassFor(props.item.status));
const statusLabel = computed(() => formatStatus(props.item.status));
const descriptionText = computed(() => props.item.description?.trim() || '—');
const lastRunTimestamp = computed(() => resolveLastRunTimestamp(props.item));

function beadClassFor(status: PipelineStatus): string {
  switch (status) {
    case 'active': return 'bg-warning';
    case 'completed':
    case 'paused': return 'bg-success';
    case 'abandoned':
    case 'stuck': return 'bg-danger';
    case 'cancelled':
    default: return 'bg-text-secondary';
  }
}

function statusTextClassFor(status: PipelineStatus): string {
  switch (status) {
    case 'active': return 'text-status-active';
    case 'completed': return 'text-status-completed';
    case 'stuck': return 'text-status-stuck';
    case 'paused': return 'text-status-ready';
    case 'abandoned':
    case 'cancelled': return 'text-status-abandoned';
    default: return 'text-status-pending';
  }
}

function progressBarClassFor(status: PipelineStatus): string {
  switch (status) {
    case 'active': return 'bg-status-active';
    case 'completed':
    case 'paused': return 'bg-status-completed';
    case 'abandoned':
    case 'stuck': return 'bg-danger';
    default: return 'bg-brand';
  }
}

function normalizeTimestamp(value?: string | number | null): { iso: string; time: number } | null {
  if (value === undefined || value === null || value === '') return null;
  const date = new Date(value);
  const time = date.getTime();
  if (Number.isNaN(time)) return null;
  return { iso: date.toISOString(), time };
}

function latestRunTimestamp(runs?: PipelineRunSummary[] | null): string | null {
  if (!Array.isArray(runs)) return null;
  let latest: { iso: string; time: number } | null = null;
  for (const run of runs) {
    const candidates = [run.timestamp, run.completedAt, run.startedAt, run.updatedAt, run.createdAt];
    for (const candidate of candidates) {
      const normalized = normalizeTimestamp(candidate);
      if (normalized && (!latest || normalized.time > latest.time)) {
        latest = normalized;
      }
    }
  }
  return latest?.iso ?? null;
}

function resolveLastRunTimestamp(item: PipelineListItem): string | null {
  return latestRunTimestamp(item.runs)
    ?? normalizeTimestamp(item.lastRunAt)?.iso
    ?? normalizeTimestamp(item.updatedAt)?.iso
    ?? normalizeTimestamp(item.modifiedAt)?.iso
    ?? null;
}

function formatLastUpdated(value?: string | number | null) {
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

function formatStatus(status: PipelineStatus): string {
  return status.charAt(0).toUpperCase() + status.slice(1);
}
</script>

<template>
  <li
    class="group flex items-start justify-between gap-3 rounded-md border-l-4 bg-bg-surface p-3 transition-colors duration-150 hover:bg-bg-surface-hover"
    :class="active ? 'border-l-brand bg-bg-surface-hover' : 'border-l-transparent'"
  >
    <Button
      type="button"
      variant="ghost"
      class="flex h-auto min-w-0 flex-1 items-start justify-start gap-2.5 rounded-md bg-transparent p-0 text-left font-normal text-text-primary hover:bg-transparent"
      :data-pipeline-id="item.pipelineId"
      data-pipeline-item
      :aria-current="active ? 'page' : undefined"
      @click="emit('select', item.pipelineId)"
    >
      <span
        class="mt-1.5 size-1.5 shrink-0 rounded-full"
        :class="beadClass"
        aria-hidden="true"
      />

      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <span
          class="truncate text-sm font-medium text-text-primary"
          :title="item.pipelineId"
        >{{ item.pipelineId }}</span>
        <span
          class="line-clamp-2 text-xs leading-4 text-text-secondary"
          :title="descriptionText"
        >{{ descriptionText }}</span>
        <span class="text-xs text-text-secondary">
          {{ item.completedCount }}/{{ item.nodeCount }} nodes
        </span>
        <span class="text-xs text-text-tertiary">
          Last run:
          <time
            v-if="lastRunTimestamp"
            :datetime="lastRunTimestamp"
            :title="lastRunTimestamp"
          >{{ formatLastUpdated(lastRunTimestamp) }}</time>
          <span v-else>—</span>
        </span>
        <Progress
          :model-value="progressPct"
          :indicator-class="progressBarClass"
          :aria-label="`${item.completedCount} of ${item.nodeCount} nodes complete`"
          class="h-1 bg-bg-app"
        />
      </div>
    </Button>

    <div class="flex min-w-0 shrink-0 flex-col items-end gap-1.5">
      <span
        class="text-xs font-medium"
        :class="statusTextClass"
      >
        {{ statusLabel }}
      </span>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        class="size-6 rounded text-text-tertiary opacity-0 transition-opacity duration-150 hover:bg-danger hover:text-white group-hover:opacity-100 group-focus-within:opacity-100"
        title="Delete pipeline"
        aria-label="Delete pipeline"
        @click="emit('delete', item, $event)"
      >
        <X class="size-3.5" aria-hidden="true" />
      </Button>
    </div>
  </li>
</template>
