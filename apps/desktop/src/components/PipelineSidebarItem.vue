<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import type { PipelineListItem, PipelineStatus } from '../lib/pipeline-types';

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
        <span class="text-xs text-text-secondary">
          {{ item.completedCount }}/{{ item.nodeCount }} nodes
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
