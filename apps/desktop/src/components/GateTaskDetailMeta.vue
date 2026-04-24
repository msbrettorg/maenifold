<script setup lang="ts">
import { computed } from 'vue';
import { AlertTriangle } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import type { GateTask } from '../lib/gate';
import { formatStageLabel, gateStageFamily, getTaskPathHint } from '../lib/gate-display';

const props = defineProps<{
  task: GateTask;
  processLabel: string;
  pipelineStale?: boolean;
}>();

const fileLabel = computed(() => getTaskPathHint(props.task.path));
const stateTone = computed(() =>
  gateStageFamily(props.task.location === 'done' ? 'done' : props.task.state),
);
const stateLabel = computed(() =>
  formatStageLabel(props.task.location === 'done' ? 'done' : props.task.state),
);
const locationLabel = computed(() => formatStageLabel(props.task.location));
const pipelineStaleTitle = computed(
  () =>
    props.task.issue?.trim() ||
    'Pipeline definition has changed since this task was created.',
);

function stateBadgeClass(tone: string): string {
  switch (tone) {
    case 'pending':
      return 'bg-text-secondary text-text-primary';
    case 'swe':
      return 'bg-brand text-text-primary';
    case 'red-team':
      return 'bg-warning text-text-primary';
    case 'blue-team':
    case 'done':
      return 'bg-success text-text-primary';
    case 'failed':
      return 'bg-danger text-text-primary';
    default:
      return 'bg-surface-highest text-text-primary';
  }
}
</script>

<template>
  <dl
    class="grid grid-cols-[repeat(auto-fit,minmax(8.5rem,1fr))] items-start gap-3"
    aria-label="Task metadata"
  >
    <div class="flex flex-col gap-1">
      <dt class="text-xs text-text-secondary">Process</dt>
      <dd class="mt-1 flex min-h-7 items-center gap-2 break-words font-mono text-xs">
        <Badge variant="secondary" class="font-mono">{{ processLabel }}</Badge>
        <span
          v-if="pipelineStale"
          role="img"
          aria-label="Pipeline definition has changed since this task was created"
          :title="pipelineStaleTitle"
          class="inline-flex cursor-help items-center text-warning"
        >
          <AlertTriangle class="h-3.5 w-3.5" aria-hidden="true" />
        </span>
      </dd>
    </div>

    <div class="flex flex-col gap-1">
      <dt class="text-xs text-text-secondary">State</dt>
      <dd class="mt-1 flex min-h-7 items-center break-words font-mono text-xs">
        <Badge :class="['font-mono', stateBadgeClass(stateTone)]">
          {{ stateLabel }}
        </Badge>
      </dd>
    </div>

    <div class="flex flex-col gap-1">
      <dt class="text-xs text-text-secondary">Location</dt>
      <dd class="mt-1 flex min-h-7 items-center break-words font-mono text-xs">
        {{ locationLabel }}
      </dd>
    </div>

    <div class="flex flex-col gap-1">
      <dt class="text-xs text-text-secondary">Attempts</dt>
      <dd class="mt-1 flex min-h-7 items-center break-words font-mono text-xs">
        {{ task.attempts }}
      </dd>
    </div>

    <div class="flex flex-col gap-1">
      <dt class="text-xs text-text-secondary">Canonical ID</dt>
      <dd class="mt-1 flex min-h-7 items-center break-words font-mono text-xs">
        {{ task.canonicalTaskId || '—' }}
      </dd>
    </div>

    <div class="col-[1/-1] flex flex-col gap-1">
      <Separator class="mb-1" />
      <dt class="text-xs text-text-secondary">File</dt>
      <dd
        :title="fileLabel"
        class="mt-1 flex min-h-7 items-center break-words font-mono text-xs"
      >
        {{ fileLabel }}
      </dd>
    </div>
  </dl>
</template>
