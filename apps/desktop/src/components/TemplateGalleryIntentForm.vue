<script setup lang="ts">
import { Input } from '@/components/ui/input';

const props = defineProps<{
  error: string;
  hasIntentAgent: boolean;
  modelValue: string;
  submitting: boolean;
}>();

const emit = defineEmits<{
  submit: [];
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <section class="gallery-hero w-full">
    <h3 class="text-lg font-semibold text-text-primary text-center mb-4">What would you like to do?</h3>
    <form class="gallery-intent-form" @submit.prevent="$emit('submit')">
      <div
        class="rounded-xl border border-border-card bg-bg-elevated px-4 py-3 shadow-[var(--shadow-8)] w-full max-w-[800px]"
      >
        <Input
          id="pipeline-intent-input"
          :model-value="props.modelValue"
          type="text"
          class="flex-1 border-0 bg-transparent text-sm text-text-primary shadow-none outline-none ring-0 placeholder:text-text-tertiary focus-visible:border-transparent focus-visible:ring-0 h-auto px-0 py-0 rounded-none w-full"
          placeholder="What do you want to do?"
          autocomplete="off"
          :disabled="props.submitting"
          :aria-invalid="props.error.length > 0"
          @update:model-value="(val: string | number) => emit('update:modelValue', String(val))"
        />
      </div>
      <p v-if="props.error" class="gallery-intent-error" role="alert">
        {{ props.error }}
      </p>
      <p class="gallery-intent-hint">
        Press Enter to start designing your pipeline.
      </p>
      <p v-if="!props.hasIntentAgent" class="gallery-agent-warning" role="alert">
        Configure an agent before creating a pipeline.
      </p>
    </form>
  </section>
</template>

<style scoped>
.gallery-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 1rem 0 0.25rem;
}

.gallery-intent-form {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
}

.gallery-intent-hint,
.gallery-intent-error,
.gallery-agent-warning {
  margin: 0;
  font-size: 0.75rem;
  text-align: center;
}

.gallery-intent-hint {
  color: var(--text-dim);
}

.gallery-intent-error,
.gallery-agent-warning {
  color: var(--terracotta);
}
</style>
