<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';

const props = defineProps<{ task: GateTask }>();

type DotTone = 'completed' | 'active' | 'failed' | 'pending';

function isFailedState(state: string) {
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

const segments = computed(() => {
  if (!props.task.stages.length) {
    return [{
      label: formatStageLabel(props.task.state),
      family: gateStageFamily(props.task.state),
      tone: isFailedState(props.task.state)
        ? 'failed'
        : props.task.location === 'done'
          ? 'completed'
          : props.task.state === 'pending'
            ? 'pending'
            : 'active',
      color: 'var(--text-secondary)',
    }];
  }

  const items = props.task.stages.map((stage) => ({
    label: formatStageLabel(stage),
    family: gateStageFamily(stage),
    tone: stageProgress(stage, props.task),
  }));

  if (isFailedState(props.task.state)) {
    items.push({ label: formatStageLabel(props.task.state), family: 'failed', tone: 'failed' });
  }

  return items;
});

const progressLabel = computed(() => formatStageLabel(props.task.location === 'done' ? 'done' : props.task.state));

function segmentBg(family: string, tone: string): string {
  if (tone === 'failed') return 'var(--terracotta)';
  if (tone === 'pending') return 'var(--surface-highest)';
  if (tone === 'completed') {
    if (family === 'pending') return 'var(--text-secondary)';
    if (family === 'swe') return 'color-mix(in srgb, var(--color-brand) 72%, var(--surface-highest))';
    if (family === 'red-team') return 'color-mix(in srgb, var(--amber) 72%, var(--surface-highest))';
    if (family === 'blue-team' || family === 'done') return 'color-mix(in srgb, var(--sage) 72%, var(--surface-highest))';
    return 'var(--surface-highest)';
  }
  if (tone === 'active') {
    if (family === 'pending') return 'var(--text-secondary)';
    if (family === 'swe') return 'var(--color-brand)';
    if (family === 'red-team') return 'var(--amber)';
    if (family === 'blue-team' || family === 'done') return 'var(--sage)';
    return 'var(--surface-highest)';
  }
  return 'var(--surface-highest)';
}
</script>

<template>
  <div class="flex flex-col items-start gap-[0.35rem]">
    <div class="flex w-full min-w-[5.5rem] gap-[0.2rem]" :aria-label="`${task.taskId} progress`">
      <!-- audit-ok: dynamic-binding (color-mix per stage tone/family) -->
      <span
        v-for="segment in segments"
        :key="`${task.taskId}-${segment.label}-${segment.tone}`"
        class="flex-1 min-w-[0.6rem] h-[0.45rem] rounded-[4px]"
        :style="{ background: segmentBg(segment.family, segment.tone) }"
        :title="segment.label"
      />
    </div>
    <span class="font-mono text-[length:var(--text-tiny)] text-[var(--color-text-secondary)]">{{ progressLabel }}</span>
  </div>
</template>
