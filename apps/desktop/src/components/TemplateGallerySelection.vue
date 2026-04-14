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
  <div class="gallery-selection">
    <div v-if="template.runInputs.length > 0" class="gallery-runtime-inputs">
      <span class="gallery-field-label">Run-time inputs</span>
      <ul class="gallery-runtime-input-list">
        <li
          v-for="input in template.runInputs"
          :key="input.key"
          class="gallery-runtime-input-item"
        >
          <span class="gallery-runtime-input-name">
            {{ input.label }}
            <span v-if="input.required" class="gallery-runtime-input-required">*</span>
          </span>
          <span v-if="input.description" class="gallery-runtime-input-desc">
            {{ input.description }}
          </span>
        </li>
      </ul>
      <p class="gallery-runtime-note">
        These values are requested when you run the pipeline, not when you create it.
      </p>
    </div>

    <div class="gallery-actions">
      <Button
        type="button"
        :disabled="!canCreate || !hasIntentAgent"
        @click="$emit('create')"
      >
        Create Pipeline
      </Button>
    </div>
  </div>
</template>

<style scoped>
.gallery-selection {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding-top: 0.5rem;
  width: min(100%, 720px);
  border-top: 1px solid var(--border);
}

.gallery-field-label {
  font-size: var(--text-tiny);
  font-weight: var(--weight-display);
  color: var(--muted-foreground);
}

.gallery-runtime-inputs {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.gallery-runtime-input-list {
  margin: 0;
  padding-left: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.gallery-runtime-input-item {
  color: var(--foreground);
  font-size: var(--text-caption);
}

.gallery-runtime-input-name {
  font-weight: var(--weight-display);
}

.gallery-runtime-input-required {
  color: var(--color-status-stuck, #e57373);
}

.gallery-runtime-input-desc,
.gallery-runtime-note {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}

.gallery-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
}
</style>
