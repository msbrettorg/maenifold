<script setup lang="ts">
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

function handleInput(event: Event): void {
  emit('update:modelValue', (event.target as HTMLInputElement).value);
}
</script>

<template>
  <section class="gallery-hero w-full">
    <h3 class="text-lg font-semibold text-text-primary text-center mb-4">What would you like to do?</h3>
    <form class="gallery-intent-form" @submit.prevent="$emit('submit')">
      <div
        class="rounded-xl border border-border-card bg-bg-elevated px-4 py-3 shadow-[var(--shadow-8)] w-full max-w-[800px]"
      >
        <input
          id="pipeline-intent-input"
          :value="props.modelValue"
          type="text"
          class="flex-1 bg-transparent text-sm text-text-primary outline-none placeholder:text-text-tertiary w-full"
          placeholder="What do you want to do?"
          autocomplete="off"
          :disabled="props.submitting"
          :aria-invalid="props.error.length > 0"
          @input="handleInput"
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
