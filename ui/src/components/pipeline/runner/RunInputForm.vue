<script setup lang="ts">
// Run-input form. When the selected pipeline's definition carries an
// `inputSchema`, this renders one typed field per schema property (string /
// number / boolean / enum), validates required fields, and emits the coerced
// `inputs` on submit. With no schema it degrades to a single optional prompt.
// All parsing/validation/coercion lives in the pure `runnerForm.ts` helpers.
import { computed, reactive, watch } from 'vue';
import { Send } from '@lucide/vue';
import type { PipelineInputs } from '../../../lib/pipeline-types';
import {
  buildInputs,
  formErrors,
  initialFormModel,
  isFormValid,
  parseInputSchema,
  type FormModel,
} from './runnerForm';

const props = defineProps<{
  schema?: Record<string, unknown> | null;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (event: 'submit', payload: { inputs?: PipelineInputs; prompt?: string }): void;
}>();

const fields = computed(() => parseInputSchema(props.schema));
const hasFields = computed(() => fields.value.length > 0);

// A single reactive model object, re-seeded whenever the field set changes.
const model = reactive<FormModel>({});
const prompt = reactive({ value: '' });

watch(
  fields,
  (next) => {
    for (const key of Object.keys(model)) delete model[key];
    Object.assign(model, initialFormModel(next));
  },
  { immediate: true }
);

const errors = computed(() => formErrors(fields.value, model));
const valid = computed(() => (hasFields.value ? isFormValid(fields.value, model) : true));

function submit(): void {
  if (props.disabled || !valid.value) return;
  if (hasFields.value) {
    emit('submit', { inputs: buildInputs(fields.value, model) });
  } else {
    const trimmed = prompt.value.trim();
    emit('submit', trimmed ? { prompt: trimmed } : {});
  }
}
</script>

<template>
  <form class="run-form" @submit.prevent="submit">
    <template v-if="hasFields">
      <div v-for="field in fields" :key="field.key" class="field">
        <label class="field-label" :for="`run-${field.key}`">
          {{ field.label }}
          <span v-if="field.required" class="req" aria-hidden="true">*</span>
        </label>
        <p v-if="field.description" class="field-desc">{{ field.description }}</p>

        <select
          v-if="field.type === 'enum'"
          :id="`run-${field.key}`"
          v-model="model[field.key] as string"
          class="control"
          :disabled="disabled"
        >
          <option value="" disabled>Select…</option>
          <option v-for="opt in field.enumValues" :key="opt" :value="opt">{{ opt }}</option>
        </select>

        <label v-else-if="field.type === 'boolean'" class="checkbox">
          <input
            :id="`run-${field.key}`"
            v-model="model[field.key] as boolean"
            type="checkbox"
            :disabled="disabled"
          />
          <span>{{ field.label }}</span>
        </label>

        <input
          v-else
          :id="`run-${field.key}`"
          v-model="model[field.key] as string"
          class="control"
          :type="field.type === 'number' ? 'number' : 'text'"
          :disabled="disabled"
        />

        <p v-if="errors[field.key]" class="field-error">{{ errors[field.key] }}</p>
      </div>
    </template>

    <div v-else class="field">
      <label class="field-label" for="run-prompt">Prompt <span class="opt">(optional)</span></label>
      <textarea
        id="run-prompt"
        v-model="prompt.value"
        class="control prompt"
        rows="2"
        placeholder="Optional kickoff prompt…"
        :disabled="disabled"
      ></textarea>
    </div>

    <button type="submit" class="run-submit" :disabled="disabled || !valid">
      <Send :size="14" :stroke-width="2" />
      <span>Run pipeline</span>
    </button>
  </form>
</template>

<style scoped>
.run-form {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.field-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.req {
  color: var(--bg-danger);
}

.opt {
  font-weight: 400;
  color: var(--text-muted);
}

.field-desc {
  margin: 0;
  font-size: 0.72rem;
  color: var(--text-muted);
}

.control {
  width: 100%;
  padding: 0.4rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-main);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  outline: none;
}

.control:focus {
  border-color: color-mix(in srgb, var(--text-accent) 45%, var(--border-color));
}

.prompt {
  resize: vertical;
}

.checkbox {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.82rem;
  color: var(--text-primary);
}

.field-error {
  margin: 0;
  font-size: 0.72rem;
  color: var(--bg-danger);
}

.run-submit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  align-self: flex-start;
  padding: 0.45rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 35%, transparent);
  border-radius: 7px;
  background: color-mix(in srgb, var(--text-accent) 14%, transparent);
  color: var(--text-accent);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, opacity 0.12s ease;
}

.run-submit:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 22%, transparent);
}

.run-submit:disabled {
  cursor: default;
  opacity: 0.45;
}
</style>
