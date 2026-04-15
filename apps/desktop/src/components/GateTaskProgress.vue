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
      color: '#707070',
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
  background: #353534;
}

.gate-progress__segment--completed.gate-progress__segment--pending { background: #707070; }
.gate-progress__segment--completed.gate-progress__segment--swe { background: color-mix(in srgb, #4d8eff 72%, #353534); }
.gate-progress__segment--completed.gate-progress__segment--red-team { background: color-mix(in srgb, #f59e0b 72%, #353534); }
.gate-progress__segment--completed.gate-progress__segment--blue-team,
.gate-progress__segment--completed.gate-progress__segment--done { background: color-mix(in srgb, #4edea3 72%, #353534); }
.gate-progress__segment--active.gate-progress__segment--pending { background: #707070; }
.gate-progress__segment--active.gate-progress__segment--swe { background: #4d8eff; }
.gate-progress__segment--active.gate-progress__segment--red-team { background: #f59e0b; }
.gate-progress__segment--active.gate-progress__segment--blue-team,
.gate-progress__segment--active.gate-progress__segment--done { background: #4edea3; }
.gate-progress__segment--pending { background: #353534; }
.gate-progress__segment--failed { background: #ffb4ab; }

.gate-progress__state {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}
</style>
