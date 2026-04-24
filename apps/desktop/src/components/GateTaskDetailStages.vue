<script setup lang="ts">
import { computed } from 'vue';
import { CheckCircle } from 'lucide-vue-next';
import type { GateTask } from '../lib/gate';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';
import {
  Stepper,
  StepperItem,
  StepperIndicator,
} from '@/components/ui/stepper';

const props = defineProps<{ task: GateTask }>();

const currentStageIndex = computed(() => {
  if (!props.task.stages.length) return -1;
  if (props.task.location === 'done') return props.task.stages.length - 1;
  return props.task.stages.indexOf(props.task.state);
});

const stagesLabelId = computed(() => `gate-task-stages-${props.task.taskId}`);

type StageState = 'completed' | 'current' | 'upcoming';

function stageState(index: number): StageState {
  if (currentStageIndex.value < 0) return 'upcoming';
  if (index < currentStageIndex.value) return 'completed';
  if (index === currentStageIndex.value) return 'current';
  return 'upcoming';
}

function itemClass(state: StageState): string {
  if (state === 'completed') return 'bg-success/10';
  if (state === 'current') return 'bg-surface-high';
  return 'bg-surface-container';
}

function trackClass(state: StageState): string {
  if (state === 'completed') return 'bg-success';
  if (state === 'current') return 'bg-brand';
  return 'bg-surface-highest';
}

function indicatorClass(state: StageState): string {
  if (state === 'completed') return 'bg-success text-white';
  if (state === 'current') return 'bg-brand text-white';
  return 'bg-surface-highest text-text-secondary';
}

function chipClass(family: string): string {
  if (family === 'swe') return 'bg-brand';
  if (family === 'red-team') return 'bg-warning';
  if (family === 'blue-team' || family === 'done') return 'bg-success';
  if (family === 'failed') return 'bg-danger';
  if (family === 'pending') return 'bg-text-secondary';
  return 'bg-surface-highest';
}
</script>

<template>
  <section
    v-if="task.stages.length"
    class="flex flex-col gap-3"
    :aria-labelledby="stagesLabelId"
  >
    <p :id="stagesLabelId" class="text-text-secondary text-xs">
      Stage progress
    </p>
    <Stepper
      :model-value="currentStageIndex + 1"
      orientation="vertical"
      class="flex-col gap-2 w-full"
    >
      <StepperItem
        v-for="(stage, index) in task.stages"
        :key="stage"
        :step="index + 1"
        :class="[
          'relative !flex-row items-stretch gap-3 rounded-md pl-2 pr-3 py-2.5 w-full overflow-hidden',
          itemClass(stageState(index)),
        ]"
      >
        <span
          aria-hidden="true"
          :class="['w-1 self-stretch rounded shrink-0', trackClass(stageState(index))]"
        />
        <StepperIndicator
          :class="[
            'rounded-md w-7 h-7 shrink-0 self-center',
            indicatorClass(stageState(index)),
          ]"
        >
          <CheckCircle
            v-if="stageState(index) === 'completed'"
            class="size-4"
            :stroke-width="1.75"
            aria-hidden="true"
          />
          <span v-else class="text-xs font-medium">{{ index + 1 }}</span>
        </StepperIndicator>
        <div class="flex flex-col gap-1 flex-1 min-w-0 self-center">
          <span
            :class="[
              'inline-flex self-start px-2 py-0.5 rounded text-xs font-mono text-text-primary',
              chipClass(gateStageFamily(stage)),
            ]"
          >
            {{ formatStageLabel(stage) }}
          </span>
          <span
            :class="[
              'text-xs inline-flex items-center gap-1',
              stageState(index) === 'current'
                ? 'text-brand font-semibold'
                : 'text-text-secondary',
            ]"
          >
            <template v-if="stageState(index) === 'current'">
              Current stage
            </template>
            <template v-else-if="stageState(index) === 'completed'">
              <CheckCircle
                class="inline size-3.5 text-success"
                :stroke-width="1.5"
                aria-hidden="true"
              />
              Complete
            </template>
            <template v-else>
              Up next
            </template>
          </span>
        </div>
      </StepperItem>
    </Stepper>
  </section>
</template>
