<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';

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
        <span class="gate-detail-stages__track" aria-hidden="true"></span>
        <span class="gate-detail-stages__index">{{ index + 1 }}</span>
        <div class="gate-detail-stages__copy">
          <span :class="['gate-detail-stages__chip', `gate-detail-stages__chip--${gateStageFamily(stage)}`]">{{ formatStageLabel(stage) }}</span>
          <span class="gate-detail-stages__status">{{ stageState(index) === 'current' ? 'Current stage' : stageState(index) === 'completed' ? 'Complete' : 'Up next' }}</span>
        </div>
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
}

.gate-detail-stages__list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.gate-detail-stages__item {
  position: relative;
  display: grid;
  grid-template-columns: auto auto 1fr;
  align-items: center;
  gap: 0.65rem;
  padding: 0.8rem 0.9rem;
  border-radius: 0.25rem;
  background: #201f1f;
}

.gate-detail-stages__track {
  width: 4px;
  align-self: stretch;
  border-radius: 999px;
  background: #353534;
}

.gate-detail-stages__index {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.4rem;
  height: 1.4rem;
  border-radius: 999px;
  background: #353534;
  font-size: var(--text-tiny);
}

.gate-detail-stages__copy {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.gate-detail-stages__chip {
  display: inline-flex;
  align-self: flex-start;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  color: #f4f7ff;
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-detail-stages__chip--pending { background: #707070; }
.gate-detail-stages__chip--swe { background: #4d8eff; }
.gate-detail-stages__chip--red-team { background: #f59e0b; }
.gate-detail-stages__chip--blue-team,
.gate-detail-stages__chip--done { background: #4edea3; }
.gate-detail-stages__chip--failed { background: #ffb4ab; }

.gate-detail-stages__status {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}

.gate-detail-stages__item--completed {
  background: color-mix(in srgb, #4edea3 8%, #201f1f);
}

.gate-detail-stages__item--completed .gate-detail-stages__track {
  background: #4edea3;
}

.gate-detail-stages__item--current {
  background: #2a2a2a;
}

.gate-detail-stages__item--current .gate-detail-stages__track {
  background: #4d8eff;
}
</style>
