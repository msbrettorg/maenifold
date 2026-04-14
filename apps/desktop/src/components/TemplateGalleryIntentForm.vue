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
  font-size: 0.75rem;
  font-weight: 400;
  color: #707070;
}

.gallery-intent-input {
  width: 100%;
  min-height: 4rem;
  padding: 1rem 1.25rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: #1c1b1b;
  color: #e5e2e1;
  font-size: clamp(1.05rem, 1rem + 0.35vw, 1.35rem);
  line-height: 1.4;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.gallery-intent-input::placeholder {
  color: #707070;
}

.gallery-intent-input:focus {
  outline: none;
  background: #2a2a2a;
  border-color: rgba(77, 142, 255, 0.4);
  box-shadow:
    0 0 0 1px rgba(77, 142, 255, 0.4),
    0 0 32px rgba(173, 198, 255, 0.08);
}

.gallery-intent-hint,
.gallery-intent-error,
.gallery-agent-warning {
  margin: 0;
  font-size: 0.75rem;
  text-align: center;
}

.gallery-intent-hint {
  color: #707070;
}

.gallery-intent-error,
.gallery-agent-warning {
  color: #ffb4ab;
}
</style>
