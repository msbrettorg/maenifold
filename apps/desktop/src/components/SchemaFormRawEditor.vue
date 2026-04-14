<script setup lang="ts">
import { Textarea } from '@/components/ui/textarea';

const props = defineProps<{
  ariaLabel?: string;
  'aria-label'?: string;
  error: string;
  message: string;
  title: string;
  value: string;
}>();

const emit = defineEmits<{
  input: [value: string];
}>();

function handleInput(event: Event): void {
  emit('input', (event.target as HTMLTextAreaElement).value);
}
</script>

<template>
  <div class="schema-form-raw-editor" role="group" :aria-label="props.ariaLabel ?? props['aria-label']">
    <div class="schema-form-raw-editor__header">
      <h3>{{ props.title }}</h3>
      <p>{{ props.message }}</p>
    </div>
    <Textarea
      class="schema-form-raw-editor__textarea"
      :value="props.value"
      rows="12"
      spellcheck="false"
      @input="handleInput"
    />
    <p v-if="props.error" class="schema-form-raw-editor__error">{{ props.error }}</p>
  </div>
</template>

<style scoped>
.schema-form-raw-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--card);
}

.schema-form-raw-editor__header {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.schema-form-raw-editor__header h3 {
  font-size: var(--text-body-sm);
  font-weight: var(--weight-title);
}

.schema-form-raw-editor__header p {
  color: var(--muted-foreground);
  font-size: var(--text-caption);
}

.schema-form-raw-editor__textarea {
  min-height: 16rem;
  font-family: var(--font-code);
}

.schema-form-raw-editor__error {
  color: var(--destructive);
  font-size: var(--text-caption);
}
</style>
