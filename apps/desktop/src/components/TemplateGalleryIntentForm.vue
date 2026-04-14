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
  <section class="gallery-hero">
    <form class="gallery-intent-form" @submit.prevent="$emit('submit')">
      <label class="gallery-intent-label" for="pipeline-intent-input">
        Start with an intent
      </label>
      <input
        id="pipeline-intent-input"
        :value="props.modelValue"
        type="text"
        class="gallery-intent-input"
        placeholder="What do you want to do?"
        autocomplete="off"
        :disabled="props.submitting"
        :aria-invalid="props.error.length > 0"
        @input="handleInput"
      />
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
  justify-content: center;
  padding: 1rem 0 0.25rem;
}

.gallery-intent-form {
  width: min(100%, 720px);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
}

.gallery-intent-label {
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
  color: var(--muted-foreground);
}

.gallery-intent-input {
  width: 100%;
  min-height: 4rem;
  padding: 1rem 1.25rem;
  border: 1px solid color-mix(in srgb, var(--color-accent, #4d8eff) 35%, var(--border));
  border-radius: 18px;
  background: color-mix(in srgb, var(--accent) 84%, var(--background));
  color: var(--foreground);
  font-size: clamp(1.05rem, 1rem + 0.35vw, 1.35rem);
  line-height: 1.4;
  box-shadow: 0 18px 40px color-mix(in srgb, var(--color-accent, #4d8eff) 10%, transparent);
}

.gallery-intent-input::placeholder {
  color: var(--muted-foreground);
}

.gallery-intent-input:focus {
  outline: none;
  border-color: var(--color-accent, #4d8eff);
  box-shadow:
    0 0 0 3px color-mix(in srgb, var(--color-accent, #4d8eff) 18%, transparent),
    0 18px 40px color-mix(in srgb, var(--color-accent, #4d8eff) 12%, transparent);
}

.gallery-intent-hint,
.gallery-intent-error,
.gallery-agent-warning {
  margin: 0;
  font-size: var(--text-caption);
  text-align: center;
}

.gallery-intent-hint {
  color: var(--muted-foreground);
}

.gallery-intent-error,
.gallery-agent-warning {
  color: var(--color-status-stuck, #e57373);
}
</style>
