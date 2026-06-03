<script setup lang="ts">
// Instantiate panel — collects a selected template's run inputs, then asks the
// gallery controller to instantiate + create the REAL pipeline. Renders one
// field per `runInputs` entry (textarea or text). Local input state only.
import { reactive } from 'vue';
import { ArrowLeft, Play } from '@lucide/vue';
import type { PipelineTemplate, TemplateInputValues } from '../../../lib/templates/pipelineTemplates';

const props = defineProps<{
  template: PipelineTemplate;
  creating: boolean;
  error?: string | null;
}>();

const emit = defineEmits<{
  (event: 'submit', values: TemplateInputValues): void;
  (event: 'back'): void;
}>();

// Seed an entry per declared input so v-model has a stable target.
const values = reactive<TemplateInputValues>(
  Object.fromEntries(props.template.runInputs.map((input) => [input.key, '']))
);

function onSubmit(): void {
  emit('submit', { ...values });
}
</script>

<template>
  <form class="inst" @submit.prevent="onSubmit">
    <header class="inst-header">
      <button type="button" class="inst-back" aria-label="Back to gallery" @click="emit('back')">
        <ArrowLeft :size="15" :stroke-width="2" />
      </button>
      <div class="inst-titles">
        <strong class="inst-title">{{ template.label }}</strong>
        <span class="inst-desc">{{ template.description }}</span>
      </div>
    </header>

    <div v-for="input in template.runInputs" :key="input.key" class="inst-field">
      <label class="inst-label" :for="`tpl-input-${input.key}`">
        {{ input.label }}<span v-if="input.required" class="inst-req" aria-hidden="true"> *</span>
      </label>
      <p v-if="input.description" class="inst-help">{{ input.description }}</p>
      <textarea
        v-if="input.type === 'textarea'"
        :id="`tpl-input-${input.key}`"
        v-model="values[input.key]"
        class="inst-input"
        rows="3"
        :placeholder="input.placeholder"
        :disabled="creating"
      ></textarea>
      <input
        v-else
        :id="`tpl-input-${input.key}`"
        v-model="values[input.key]"
        type="text"
        class="inst-input"
        :placeholder="input.placeholder"
        :disabled="creating"
      />
    </div>

    <p v-if="error" class="inst-error" role="status">{{ error }}</p>

    <div class="inst-actions">
      <button type="submit" class="inst-create" :disabled="creating">
        <Play :size="15" :stroke-width="2" />
        <span>{{ creating ? 'Creating…' : 'Create pipeline' }}</span>
      </button>
    </div>
  </form>
</template>

<style scoped>
.inst {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  padding: 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
}

.inst-header {
  display: flex;
  align-items: flex-start;
  gap: 0.55rem;
}

.inst-back {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0.3rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.inst-back:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.inst-titles {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
}

.inst-title {
  font-size: 0.9rem;
  font-weight: 600;
}

.inst-desc {
  font-size: 0.78rem;
  color: var(--text-secondary);
  line-height: 1.35;
}

.inst-field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.inst-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.inst-req {
  color: var(--text-accent);
}

.inst-help {
  margin: 0;
  font-size: 0.73rem;
  color: var(--text-muted);
}

.inst-input {
  width: 100%;
  resize: vertical;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  line-height: 1.4;
  outline: none;
}

.inst-input:focus {
  border-color: color-mix(in srgb, var(--text-accent) 45%, var(--border-color));
}

.inst-error {
  margin: 0;
  padding: 0.4rem 0.5rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.76rem;
}

.inst-actions {
  display: flex;
  justify-content: flex-end;
}

.inst-create {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.45rem 0.85rem;
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

.inst-create:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 22%, transparent);
}

.inst-create:disabled {
  cursor: default;
  opacity: 0.55;
}
</style>
