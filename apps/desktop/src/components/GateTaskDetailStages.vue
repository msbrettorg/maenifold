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

function stageState(index: number): 'completed' | 'current' | 'upcoming' {
  if (currentStageIndex.value < 0) return 'upcoming';
  if (index < currentStageIndex.value) return 'completed';
  if (index === currentStageIndex.value) return 'current';
  return 'upcoming';
}

function itemBg(state: string): string {
  if (state === 'completed') return 'color-mix(in srgb, var(--sage) 8%, var(--surface-container))';
  if (state === 'current') return 'var(--surface-high)';
  return 'var(--surface-container)';
}

function trackBg(state: string): string {
  if (state === 'completed') return 'var(--sage)';
  if (state === 'current') return 'var(--color-brand)';
  return 'var(--surface-highest)';
}

function chipBg(family: string): string {
  if (family === 'swe') return 'var(--color-brand)';
  if (family === 'red-team') return 'var(--amber)';
  if (family === 'blue-team' || family === 'done') return 'var(--sage)';
  if (family === 'failed') return 'var(--terracotta)';
  if (family === 'pending') return 'var(--text-secondary)';
  return 'var(--surface-highest)';
}
</script>

<template>
  <section v-if="task.stages.length" class="flex flex-col gap-3" :aria-labelledby="stagesLabelId">
    <p :id="stagesLabelId" class="text-text-secondary text-[length:var(--text-tiny)]">Stage progress</p>
    <ol class="flex flex-col gap-3 m-0 p-0 list-none">
      <!-- audit-ok: dynamic-binding (item background varies by completion state, including color-mix) -->
      <li
        v-for="(stage, index) in task.stages"
        :key="stage"
        class="relative grid grid-cols-[auto_auto_1fr] items-center gap-[0.65rem] px-[0.9rem] py-[0.8rem] rounded"
        :style="{ background: itemBg(stageState(index)) }"
      >
        <!-- audit-ok: dynamic-binding (track color varies by completion state) -->
        <span
          class="w-1 self-stretch rounded"
          aria-hidden="true"
          :style="{ background: trackBg(stageState(index)) }"
        ></span>
        <span class="inline-flex items-center justify-center w-[1.4rem] h-[1.4rem] rounded-[4px] bg-surface-highest text-[length:var(--text-tiny)]">{{ index + 1 }}</span>
        <div class="flex flex-col gap-[0.3rem]">
          <!-- audit-ok: dynamic-binding (chip color varies by stage family) -->
          <span
            class="inline-flex self-start px-[0.55rem] py-[0.2rem] rounded-[4px] text-text-primary font-mono text-[length:var(--text-tiny)]"
            :style="{ background: chipBg(gateStageFamily(stage)) }"
          >{{ formatStageLabel(stage) }}</span>
          <span
            :class="[
              'text-text-secondary text-[length:var(--text-tiny)]',
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
