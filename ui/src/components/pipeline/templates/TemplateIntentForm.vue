<script setup lang="ts">
// Intent track — a free-form "describe what you want" textarea that creates a
// one-node pipeline. Owns only its local intent text; the create flow (resolve
// agent → build → validate → create) lives in the gallery controller, invoked
// via the `submit` emit.
import { ref } from 'vue';
import { Wand2 } from '@lucide/vue';

defineProps<{
  creating: boolean;
  disabled: boolean;
  disabledReason?: string | null;
  error?: string | null;
}>();

const emit = defineEmits<{ (event: 'submit', intent: string): void }>();

const intent = ref('');

function onSubmit(): void {
  emit('submit', intent.value);
}
</script>

<template>
  <form class="intent" @submit.prevent="onSubmit">
    <label class="intent-label" for="template-intent">Describe what you want</label>
    <textarea
      id="template-intent"
      v-model="intent"
      class="intent-input"
      rows="3"
      placeholder="e.g. Research the tradeoffs between approach A and B, then critique the result"
      :disabled="creating"
    ></textarea>
    <p v-if="disabled && disabledReason" class="intent-note">{{ disabledReason }}</p>
    <p v-if="error" class="intent-error" role="status">{{ error }}</p>
    <div class="intent-actions">
      <button type="submit" class="intent-create" :disabled="creating || disabled || !intent.trim()">
        <Wand2 :size="15" :stroke-width="2" />
        <span>{{ creating ? 'Creating…' : 'Create from intent' }}</span>
      </button>
    </div>
  </form>
</template>

<style scoped>
.intent {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
}

.intent-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.intent-input {
  width: 100%;
  resize: vertical;
  padding: 0.55rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  line-height: 1.4;
  outline: none;
}

.intent-input:focus {
  border-color: color-mix(in srgb, var(--text-accent) 45%, var(--border-color));
}

.intent-note {
  margin: 0;
  font-size: 0.74rem;
  color: var(--text-muted);
}

.intent-error {
  margin: 0;
  padding: 0.4rem 0.5rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.76rem;
}

.intent-actions {
  display: flex;
  justify-content: flex-end;
}

.intent-create {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.45rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 40%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--text-accent) 14%, transparent);
  color: var(--text-accent);
  cursor: pointer;
  font: inherit;
  font-size: 0.8rem;
  font-weight: 600;
  transition: background 0.12s ease;
}

.intent-create:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 22%, transparent);
}

.intent-create:disabled {
  cursor: default;
  opacity: 0.55;
}
</style>
