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
</script>

<template>
  <div class="gate-progress">
    <div class="gate-progress__bar" :aria-label="`${task.taskId} progress`">
      <span
        v-for="segment in segments"
        :key="`${task.taskId}-${segment.label}-${segment.tone}`"
        class="gate-progress__segment"
        :class="[`gate-progress__segment--${segment.tone}`, `gate-progress__segment--${segment.family}`]"
        :title="segment.label"
      />
    </div>
    <span class="gate-progress__state">{{ progressLabel }}</span>
  </div>
</template>

<style scoped>
.gate-progress {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.35rem;
}

.gate-progress__bar {
  display: flex;
  width: 100%;
  min-width: 5.5rem;
  gap: 0.2rem;
}

.gate-progress__state {
  font-family: var(--font-code);
}

.gate-progress__segment {
  flex: 1;
  min-width: 0.6rem;
  height: 0.45rem;
  border-radius: 4px;
  background: var(--surface-highest);
}

.gate-progress__segment--completed.gate-progress__segment--pending { background: var(--text-secondary); }
.gate-progress__segment--completed.gate-progress__segment--swe { background: color-mix(in srgb, var(--color-brand) 72%, var(--surface-highest)); }
.gate-progress__segment--completed.gate-progress__segment--red-team { background: color-mix(in srgb, var(--amber) 72%, var(--surface-highest)); }
.gate-progress__segment--completed.gate-progress__segment--blue-team,
.gate-progress__segment--completed.gate-progress__segment--done { background: color-mix(in srgb, var(--sage) 72%, var(--surface-highest)); }
.gate-progress__segment--active.gate-progress__segment--pending { background: var(--text-secondary); }
.gate-progress__segment--active.gate-progress__segment--swe { background: var(--color-brand); }
.gate-progress__segment--active.gate-progress__segment--red-team { background: var(--amber); }
.gate-progress__segment--active.gate-progress__segment--blue-team,
.gate-progress__segment--active.gate-progress__segment--done { background: var(--sage); }
.gate-progress__segment--pending { background: var(--surface-highest); }
.gate-progress__segment--failed { background: var(--terracotta); }

.gate-progress__state {
  color: var(--color-text-secondary);
  font-size: var(--text-tiny);
}
</style>
