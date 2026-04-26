<script setup lang="ts">
import { computed } from 'vue';
import type { GateTask } from '../lib/gate';
import { formatStageLabel } from '../lib/gate-display';
import { Progress } from './ui/progress';
import { Badge } from './ui/badge';

const props = defineProps<{ task: GateTask }>();

const stageCount = computed(() => props.task.stages.length || 1);

const completedCount = computed(() => {
  if (props.task.location === 'done') return stageCount.value;
  if (!props.task.stages.length) return 0;
  const idx = props.task.stages.indexOf(props.task.state);
  return idx >= 0 ? idx : 0;
});

const percent = computed(() =>
  Math.round((completedCount.value / stageCount.value) * 100),
);

const stageLabel = computed(() => {
  if (props.task.location === 'done') return 'Done';
  return formatStageLabel(props.task.state);
});

const progressLabel = computed(() =>
  `${completedCount.value}/${stageCount.value}`,
);
</script>

<template>
  <div
    class="flex items-center gap-2"
    :aria-label="`${task.taskId} progress: ${stageLabel} (${progressLabel})`"
  >
    <Progress
      :model-value="percent"
      class="h-1.5 w-16"
      :indicator-class="task.location === 'done' ? 'bg-status-completed' : 'bg-status-active'"
    />
    <Badge variant="secondary" class="whitespace-nowrap px-1.5 py-0 text-[10px] font-medium">
      {{ stageLabel }}
    </Badge>
  </div>
</template>
