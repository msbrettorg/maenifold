<script setup lang="ts">
import { Button } from '@/components/ui/button';
import type { PipelineTemplate } from '../lib/pipeline-templates';

defineProps<{
  canCreate: boolean;
  hasIntentAgent: boolean;
  template: PipelineTemplate;
}>();

defineEmits<{
  create: [];
}>();
</script>

<template>
  <div class="flex flex-col gap-3 p-4 w-[min(100%,720px)] border border-border-subtle rounded-[0.25rem] bg-surface">
    <div v-if="template.runInputs.length > 0" class="flex flex-col gap-[0.4rem]">
      <span class="text-[0.75rem] font-normal text-text-secondary">Run-time inputs</span>
      <ul class="m-0 pl-4 flex flex-col gap-1">
        <li
          v-for="input in template.runInputs"
          :key="input.key"
          class="text-text-primary text-[0.875rem]"
        >
          <span class="font-medium">
            {{ input.label }}
            <span v-if="input.required" class="text-danger">*</span>
          </span>
          <span v-if="input.description" class="text-text-secondary text-[0.75rem]">
            {{ input.description }}
          </span>
        </li>
      </ul>
      <p class="text-text-secondary text-[0.75rem]">
        These values are requested when you run the pipeline, not when you create it.
      </p>
    </div>

    <div class="flex items-center justify-center gap-3">
      <Button
        type="button"
        :disabled="!canCreate || !hasIntentAgent"
        @click="$emit('create')"
      >
        Create pipeline
      </Button>
    </div>
  </div>
</template>
