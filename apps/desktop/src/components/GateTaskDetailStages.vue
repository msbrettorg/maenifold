<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';

const props = defineProps<{ task: GateTask }>();

const currentStageIndex = computed(() => {
  if (!props.task.stages.length) return -1;
  if (props.task.location === 'done') return props.task.stages.length - 1;
  return props.task.stages.indexOf(props.task.state);
});

const stagesLabelId = computed(() => `gate-task-stages-${props.task.taskId}`);

function stageState(index: number) {
  if (currentStageIndex.value < 0) return 'upcoming';
  if (index < currentStageIndex.value) return 'completed';
  if (index === currentStageIndex.value) return 'current';
  return 'upcoming';
}
</script>

<template>
  <section v-if="task.stages.length" class="gate-detail-stages" :aria-labelledby="stagesLabelId">
    <p :id="stagesLabelId" class="gate-detail-stages__label">Stage progress</p>
    <ol class="gate-detail-stages__list">
      <li
        v-for="(stage, index) in task.stages"
        :key="stage"
        :class="['gate-detail-stages__item', `gate-detail-stages__item--${stageState(index)}`]"
      >
        <span class="gate-detail-stages__index">{{ index + 1 }}</span>
        <span class="gate-detail-stages__name">{{ stage }}</span>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.gate-detail-stages {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.gate-detail-stages__label {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.gate-detail-stages__list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.gate-detail-stages__item {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.45rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--muted);
  color: var(--muted-foreground);
  font-family: var(--font-code);
  font-size: var(--text-caption);
}

.gate-detail-stages__index {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--card) 78%, var(--muted));
  font-size: var(--text-tiny);
}

.gate-detail-stages__item--completed {
  border-color: color-mix(in srgb, var(--color-status-completed) 35%, var(--border));
  color: var(--color-status-completed);
}

.gate-detail-stages__item--current {
  border-color: color-mix(in srgb, var(--color-status-active) 45%, var(--border));
  background: color-mix(in srgb, var(--color-status-active) 10%, var(--muted));
  color: var(--foreground);
}
</style>
