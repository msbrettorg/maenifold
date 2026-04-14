<script setup lang="ts">
import type { PipelineNodeType } from '../lib/pipeline-types';

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
        class="field-input"
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
      <button type="submit" class="btn btn-primary" :disabled="!canAdd">
        {{ submitLabel }}
      </button>
    </div>
  </div>
</template>
