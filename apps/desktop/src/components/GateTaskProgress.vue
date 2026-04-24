<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { Skeleton } from '@/components/ui/skeleton';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';
import { Badge } from './ui/badge';

const props = defineProps<{ task: GateTask }>();

type DotTone = 'completed' | 'active' | 'failed' | 'pending';
type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline';

interface Segment {
  label: string;
  family: string;
  tone: DotTone;
}

function isFailedState(state: string): boolean {
  return state === 'error' || state === 'failed';
}

function stageProgress(stage: string, task: GateTask): DotTone {
  if (task.location === 'done') return 'completed';

  const stageIndex = task.stages.indexOf(stage);

  if (isFailedState(task.state)) {
    let failedIndex = -1;
    for (let index = task.stages.length - 1; index >= 0; index -= 1) {
      if (!['done', 'completed'].includes(task.stages[index])) {
        failedIndex = index;
        break;
      }
    }
    if (failedIndex < 0) return 'pending';
    if (stageIndex < failedIndex) return 'completed';
    if (stageIndex === failedIndex) return 'failed';
    return 'pending';
  }

  const activeIndex = task.stages.indexOf(task.state);
  if (activeIndex < 0) return 'pending';
  if (stageIndex < activeIndex) return 'completed';
  if (stageIndex === activeIndex) return 'active';
  return 'pending';
}

const segments = computed<Segment[]>(() => {
  if (!props.task.stages.length) {
    const tone: DotTone = isFailedState(props.task.state)
      ? 'failed'
      : props.task.location === 'done'
        ? 'completed'
        : props.task.state === 'pending'
          ? 'pending'
          : 'active';
    return [{
      label: formatStageLabel(props.task.state),
      family: gateStageFamily(props.task.state),
      tone,
    }];
  }

  const items: Segment[] = props.task.stages.map((stage) => ({
    label: formatStageLabel(stage),
    family: gateStageFamily(stage),
    tone: stageProgress(stage, props.task),
  }));

  if (isFailedState(props.task.state)) {
    items.push({
      label: formatStageLabel(props.task.state),
      family: 'failed',
      tone: 'failed',
    });
  }

  return items;
});

const progressLabel = computed(() =>
  formatStageLabel(props.task.location === 'done' ? 'done' : props.task.state),
);

function badgeVariant(tone: DotTone): BadgeVariant {
  if (tone === 'failed') return 'destructive';
  if (tone === 'completed') return 'default';
  if (tone === 'active') return 'default';
  return 'outline';
}

function activeSkeletonWidth(label: string): string {
  return `${Math.max(label.length + 2, 6)}ch`;
}
</script>

<template>
  <div class="flex flex-col items-start gap-1.5">
    <div
      class="flex flex-wrap items-center gap-1"
      :aria-label="`${task.taskId} progress`"
    >
      <template
        v-for="segment in segments"
        :key="`${task.taskId}-${segment.label}-${segment.tone}`"
      >
        <Skeleton
          v-if="segment.tone === 'active'"
          class="h-6 rounded-md"
          :style="{ width: activeSkeletonWidth(segment.label) }"
          :title="segment.label"
        />
        <Badge
          v-else
          :variant="badgeVariant(segment.tone)"
          :class="segment.tone === 'pending' ? 'opacity-60' : ''"
          :title="segment.label"
        >
          {{ segment.label }}
        </Badge>
      </template>
    </div>
    <span class="font-mono text-xs text-muted-foreground">{{ progressLabel }}</span>
  </div>
</template>
