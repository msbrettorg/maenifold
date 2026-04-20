<script setup lang="ts">
import { computed } from 'vue';
import { CheckCircle } from 'lucide-vue-next';
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
          <span
            :class="[
              'gate-detail-stages__status',
              stageState(index) === 'current' ? 'text-brand font-semibold' : '',
            ]"
          >
            <template v-if="stageState(index) === 'current'">
              Current stage
            </template>
            <template v-else-if="stageState(index) === 'completed'">
              <CheckCircle class="inline size-3.5 text-success" :stroke-width="1.5" />
              Complete
            </template>
            <template v-else>
              Up next
            </template>
          </span>
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
  color: var(--color-text-secondary);
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
  background: var(--surface-container);
}

.gate-detail-stages__track {
  width: 4px;
  align-self: stretch;
  border-radius: 4px;
  background: var(--surface-highest);
}

.gate-detail-stages__index {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.4rem;
  height: 1.4rem;
  border-radius: 4px;
  background: var(--surface-highest);
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
  border-radius: 4px;
  color: var(--color-text-primary);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-detail-stages__chip--pending { background: var(--text-secondary); }
.gate-detail-stages__chip--swe { background: var(--color-brand); }
.gate-detail-stages__chip--red-team { background: var(--amber); }
.gate-detail-stages__chip--blue-team,
.gate-detail-stages__chip--done { background: var(--sage); }
.gate-detail-stages__chip--failed { background: var(--terracotta); }

.gate-detail-stages__status {
  color: var(--color-text-secondary);
  font-size: var(--text-tiny);
}

.gate-detail-stages__item--completed {
  background: color-mix(in srgb, var(--sage) 8%, var(--surface-container));
}

.gate-detail-stages__item--completed .gate-detail-stages__track {
  background: var(--sage);
}

.gate-detail-stages__item--current {
  background: var(--surface-high);
}

.gate-detail-stages__item--current .gate-detail-stages__track {
  background: var(--color-brand);
}
</style>
