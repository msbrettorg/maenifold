<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { PipelineNodeType } from '../lib/pipeline-types';

defineProps<{
  modelValue: PipelineNodeType;
  canAdd: boolean;
  submitLabel: string;
}>();

const emit = defineEmits<{
  (event: 'update:modelValue', value: PipelineNodeType): void;
}>();
</script>

<template>
  <div class="toolbar-row toolbar-row-top">
    <div class="toolbar-type-field">
      <Label>Node type</Label>
      <Select :model-value="modelValue" @update:model-value="(v) => emit('update:modelValue', String(v) as PipelineNodeType)">
        <SelectTrigger aria-label="Node type">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="workflow">Workflow</SelectItem>
          <SelectItem value="prompt">Prompt</SelectItem>
          <SelectItem value="sequentialThinking">Sequential Thinking</SelectItem>
          <SelectItem value="tool">Tool</SelectItem>
        </SelectContent>
      </Select>
    </div>
    <div class="toolbar-actions">
      <Button type="submit" :disabled="!canAdd">{{ submitLabel }}</Button>
    </div>
  </div>
</template>
