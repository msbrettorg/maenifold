<script setup lang="ts">
import WorkflowStepCard from './WorkflowStepCard.vue';
import { Badge } from '@/components/ui/badge';
import type { WorkflowStepDefinition } from '../lib/workflow-asset-types';

const props = defineProps<{
  step: WorkflowStepDefinition;
  index: number;
  totalSteps: number;
  selectedStepId: string | null;
  expanded: boolean;
  referenceWarning: string | null;
}>();

const emit = defineEmits<{
  select: [stepId: string];
  'toggle-expanded': [stepId: string];
  'move-up': [stepId: string];
  'move-down': [stepId: string];
  remove: [stepId: string];
}>();

function formatStepNumber(index: number): string {
  return String(index + 1).padStart(2, '0');
}
</script>

<template>
  <li class="flex gap-3 items-stretch">
    <div class="flex flex-col items-center w-10 shrink-0">
      <Badge
        variant="outline"
        :class="[
          'flex items-center justify-center w-10 h-10 rounded font-mono text-xs font-semibold shrink-0 transition-colors duration-150',
          props.selectedStepId === step.id
            ? 'border-node-workflow text-node-workflow bg-node-workflow/10'
            : 'border-border-subtle text-text-secondary bg-bg-elevated',
        ]"
      >
        {{ formatStepNumber(index) }}
      </Badge>
      <span
        v-if="index < totalSteps - 1"
        class="flex-1 w-px min-h-3 border-l-2 border-dashed border-border-subtle"
      />
    </div>

    <WorkflowStepCard
      :can-remove="totalSteps > 1"
      :can-move-down="index < totalSteps - 1"
      :can-move-up="index > 0"
      :expanded="expanded"
      :reference-warning="referenceWarning"
      :selected="props.selectedStepId === step.id"
      :step="step"
      @move-down="(id: string) => emit('move-down', id)"
      @move-up="(id: string) => emit('move-up', id)"
      @remove="(id: string) => emit('remove', id)"
      @select="(id: string) => emit('select', id)"
      @toggle-expanded="(id: string) => emit('toggle-expanded', id)"
    />
  </li>
</template>
