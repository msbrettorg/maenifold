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
        Create pipeline
      </Button>
    </div>
  </div>
</template>

<style scoped>
.gallery-selection {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem;
  width: min(100%, 720px);
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: #1c1b1b;
}

.gallery-field-label {
  font-size: 0.75rem;
  font-weight: 400;
  color: #707070;
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
  color: #e5e2e1;
  font-size: 0.875rem;
}

.gallery-runtime-input-name {
  font-weight: var(--weight-display);
}

.gallery-runtime-input-required {
  color: #ffb4ab;
}

.gallery-runtime-input-desc,
.gallery-runtime-note {
  color: #c2c6d6;
  font-size: 0.75rem;
}

.gallery-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
}
</style>
