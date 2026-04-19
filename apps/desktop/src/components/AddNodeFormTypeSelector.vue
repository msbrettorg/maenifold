<script setup lang="ts">
import { Button } from '@/components/ui/button';
import type { PipelineNodeType } from '../lib/pipeline-types';

const nativeSelectClass =
  'w-full bg-[var(--surface)] text-foreground border border-[rgba(66,71,83,0.15)] rounded-[4px] px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20';

defineProps<{
  modelValue: PipelineNodeType;
  canAdd: boolean;
  submitLabel: string;
}>();

const emit = defineEmits<{
  (event: 'update:modelValue', value: PipelineNodeType): void;
}>();

function updateNodeType(event: Event): void {
  emit('update:modelValue', (event.target as HTMLSelectElement).value as PipelineNodeType);
}
</script>

<template>
  <div class="toolbar-row toolbar-row-top">
    <label class="toolbar-type-field">
      <span>Node type</span>
      <select
        :class="nativeSelectClass"
        aria-label="Node type"
        :value="modelValue"
        @change="updateNodeType"
      >
        <option value="workflow">Workflow</option>
        <option value="prompt">Prompt</option>
        <option value="sequentialThinking">Sequential Thinking</option>
        <option value="tool">Tool</option>
      </select>
    </label>
    <div class="toolbar-actions">
      <Button type="submit" :disabled="!canAdd">{{ submitLabel }}</Button>
    </div>
  </div>
</template>
