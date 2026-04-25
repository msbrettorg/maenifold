<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { formatStageLabel } from '../lib/gate-display';

const props = defineProps<{ task: GateTask }>();

type DotTone = 'completed' | 'active' | 'pending';

interface Segment {
  label: string;
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
    if (stageIndex === failedIndex) return 'active';
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
      ? 'active'
      : props.task.location === 'done'
        ? 'completed'
        : props.task.state === 'pending'
          ? 'pending'
          : 'active';
    return [{
      label: formatStageLabel(props.task.state),
      tone,
    }];
  }

  const items: Segment[] = props.task.stages.map((stage) => ({
    label: formatStageLabel(stage),
    tone: stageProgress(stage, props.task),
  }));

  return items;
});

function dotClass(tone: DotTone): string {
  if (tone === 'completed') return 'bg-status-completed';
  if (tone === 'active') return 'bg-status-active';
  return 'bg-status-pending';
}
</script>

<template>
  <div class="flex flex-col items-start gap-1.5">
    <div
      class="flex flex-wrap items-start gap-1.5"
      :aria-label="`${task.taskId} progress`"
    >
      <div
        v-for="segment in segments"
        :key="`${task.taskId}-${segment.label}-${segment.tone}`"
        class="flex min-w-3 flex-col items-center gap-1"
      >
        <span
          class="h-1.5 w-1.5 rounded-full"
          :class="[dotClass(segment.tone), segment.tone === 'active' ? 'animate-pulse' : '']"
          :title="segment.label"
          aria-hidden="true"
        />
        <span
          v-if="segment.tone === 'active'"
          class="whitespace-nowrap font-mono text-[10px] leading-none text-muted-foreground"
        >{{ segment.label }}</span>
      </div>
    </div>
  </div>
</template>
