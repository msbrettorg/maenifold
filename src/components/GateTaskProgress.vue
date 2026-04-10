<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';

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

const dots = computed(() => {
  if (!props.task.stages.length) {
    return [{
      label: props.task.state,
      symbol: isFailedState(props.task.state) ? '✗' : props.task.state === 'pending' ? '○' : '●',
      tone: isFailedState(props.task.state)
        ? 'failed'
        : props.task.location === 'done'
          ? 'completed'
          : props.task.state === 'pending'
            ? 'pending'
            : 'active',
    }];
  }

  const items = props.task.stages.map((stage) => ({
    label: stage.replace(/-/g, ' '),
    symbol: stageProgress(stage, props.task) === 'active' ? '◉' : stageProgress(stage, props.task) === 'pending' ? '○' : '●',
    tone: stageProgress(stage, props.task),
  }));

  if (isFailedState(props.task.state)) {
    items.push({ label: props.task.state.replace(/-/g, ' '), symbol: '✗', tone: 'failed' });
  }

  return items;
});
</script>

<template>
  <div class="gate-progress">
    <div class="gate-progress__dots" :aria-label="`${task.taskId} progress`">
      <span
        v-for="dot in dots"
        :key="`${task.taskId}-${dot.label}-${dot.symbol}`"
        class="gate-progress__dot"
        :class="`gate-progress__dot--${dot.tone}`"
        :title="dot.label"
      >
        {{ dot.symbol }}
      </span>
    </div>
    <span class="gate-progress__state">{{ task.state }}</span>
  </div>
</template>

<style scoped>
.gate-progress,
.gate-progress__dots {
  display: flex;
  align-items: center;
  gap: 0.2rem;
}

.gate-progress {
  flex-direction: column;
  align-items: flex-start;
}

.gate-progress__dot,
.gate-progress__state {
  font-family: var(--font-mono);
}

.gate-progress__dot--completed { color: var(--color-status-completed); }
.gate-progress__dot--active { color: var(--color-status-active); }
.gate-progress__dot--failed { color: var(--color-status-abandoned); }
.gate-progress__dot--pending { color: var(--text-muted); }

.gate-progress__state {
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: lowercase;
}
</style>
