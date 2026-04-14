<script setup lang="ts">
import { JsonForms } from '@jsonforms/vue';
import { computed, provide, ref, watch } from 'vue';
import { frozenRenderers, themedStyles } from '../lib/jsonforms-theme';
import {
  cloneRecord,
  getFormValidationErrors,
  inspectFormSchema,
  isRecord,
  normalizeFormData,
  schemaFormAjv,
  stringifyRecord,
  validateFormData,
} from '../lib/schema-form';

const props = defineProps<{
  schema: unknown;
  data: Record<string, unknown>;
}>();

const emit = defineEmits<{
  change: [data: Record<string, unknown>];
  validation: [state: { valid: boolean }];
}>();

const rawJson = ref('');
const rawJsonError = ref('');
const showAdvancedRaw = ref(false);
const lastPublishedData = ref(stringifyRecord(props.data));

provide('styles', themedStyles);

const schemaAssessment = computed(() => inspectFormSchema(props.schema));
const fallbackReason = computed(() => schemaAssessment.value.fallbackReason);
const fallbackMessage = computed(() => {
  switch (fallbackReason.value) {
    case 'complex':
      return 'Schema is too large or deeply nested for the inspector — edit the payload directly.';
    case 'unsafe':
      return 'Schema uses external references that the inspector will not resolve — edit the payload directly.';
    case 'unsupported':
      return 'Schema shape is not supported in the inspector — edit the payload directly.';
    default:
      return 'Schema unavailable — edit the payload directly.';
  }
});
const rawEditorTitle = computed(() => (fallbackReason.value === null ? 'Advanced Raw JSON' : 'Raw JSON'));
const rawEditorMessage = computed(() =>
  fallbackReason.value === null
    ? 'Advanced mode bypasses the form renderer and edits the payload directly.'
    : fallbackMessage.value,
);
const canToggleAdvancedRaw = computed(() => fallbackReason.value === null);
const showRawEditor = computed(() => fallbackReason.value !== null || showAdvancedRaw.value);
const formSchema = computed(() => schemaAssessment.value.formSchema);
const formValidator = computed(() => schemaAssessment.value.validator);

watch(
  () => props.data,
  (value) => {
    const nextSerialized = stringifyRecord(value);
    lastPublishedData.value = nextSerialized;
    if (!showRawEditor.value) {
      emitFormValidation();
      return;
    }

    const nextRawJson = nextSerialized;
    if (nextRawJson !== rawJson.value) {
      rawJson.value = nextRawJson;
    }
    emitRawValidation(normalizeFormData(value));
  },
  { deep: true, immediate: true },
);

watch(
  () => fallbackReason.value,
  (reason) => {
    if (reason !== null) {
      showAdvancedRaw.value = false;
    }
  },
  { immediate: true },
);

watch(
  () => [props.schema, showRawEditor.value] as const,
  () => {
    if (showRawEditor.value) {
      rawJson.value = stringifyRecord(props.data);
      lastPublishedData.value = rawJson.value;
      emitRawValidation(normalizeFormData(props.data));
      return;
    }

    emitFormValidation();
  },
  { immediate: true },
);

function handleFormChange(event: { data: unknown; errors: unknown[] }): void {
  emit('validation', { valid: event.errors.length === 0 });
  const nextData = normalizeFormData(event.data);
  const nextSerialized = stringifyRecord(nextData);
  if (nextSerialized === lastPublishedData.value) {
    return;
  }
  lastPublishedData.value = nextSerialized;
  emit('change', nextData);
}

function handleRawInput(event: Event): void {
  rawJson.value = (event.target as HTMLTextAreaElement).value;

  try {
    const parsed = JSON.parse(rawJson.value) as unknown;
    if (!isRecord(parsed)) {
      rawJsonError.value = 'Fallback JSON must parse to an object.';
      emit('validation', { valid: false });
      return;
    }

    const nextData = cloneRecord(parsed);
    if (!emitRawValidation(nextData)) {
      return;
    }
    const nextSerialized = stringifyRecord(nextData);
    if (nextSerialized === lastPublishedData.value) {
      return;
    }
    lastPublishedData.value = nextSerialized;
    emit('change', nextData);
  } catch {
    rawJsonError.value = 'Fallback JSON must be valid JSON.';
    emit('validation', { valid: false });
  }
}

function toggleAdvancedRaw(): void {
  if (!canToggleAdvancedRaw.value) {
    return;
  }
  showAdvancedRaw.value = !showAdvancedRaw.value;
}

function emitFormValidation(): void {
  emit('validation', {
    valid: validateFormData(formValidator.value, normalizeFormData(props.data)),
  });
}

function emitRawValidation(data: Record<string, unknown>): boolean {
  const errors = getFormValidationErrors(formValidator.value, data);

  if (errors == null) {
    // No schema available — allow the change through without blocking.
    rawJsonError.value = '';
    emit('validation', { valid: true });
    return true;
  }

  const valid = errors.length === 0;
  rawJsonError.value = valid ? '' : formatValidationError(errors[0]);
  emit('validation', { valid });
  return valid;
}

function formatValidationError(error: { instancePath?: string; message?: string; keyword?: string }): string {
  const location = error.instancePath?.trim() ? error.instancePath : '/';
  const message = error.message?.trim() || 'Schema validation failed.';
  return `${location}: ${message}`;
}
</script>

<template>
  <section class="schema-form">
    <div class="schema-form__surface">
      <div v-if="canToggleAdvancedRaw" class="schema-form__toolbar">
        <button type="button" class="btn btn-ghost" @click="toggleAdvancedRaw">
          {{ showAdvancedRaw ? 'Back to Form' : 'Advanced JSON' }}
        </button>
      </div>
      <div v-if="showRawEditor" class="schema-form__fallback panel">
        <div class="schema-form__fallback-header">
          <h3>{{ rawEditorTitle }}</h3>
          <p>{{ rawEditorMessage }}</p>
        </div>
        <textarea
          class="field-textarea schema-form__fallback-textarea"
          :value="rawJson"
          rows="12"
          spellcheck="false"
          @input="handleRawInput"
        />
        <p v-if="rawJsonError" class="schema-form__fallback-error">{{ rawJsonError }}</p>
      </div>
      <json-forms
        v-else
        :ajv="schemaFormAjv"
        :data="props.data"
        :renderers="frozenRenderers"
        :schema="formSchema ?? undefined"
        validation-mode="ValidateAndShow"
        @change="handleFormChange"
      />
    </div>
  </section>
</template>

<style scoped>
.schema-form,
.schema-form__surface,
.schema-form__fallback {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.schema-form__surface {
  max-height: min(32rem, 60vh);
  overflow-y: auto;
  padding-right: var(--space-1);
  scrollbar-width: thin;
  scrollbar-color: var(--border-default) transparent;
}

.schema-form__toolbar {
  display: flex;
  justify-content: flex-end;
}

.schema-form__surface::-webkit-scrollbar {
  width: 8px;
}

.schema-form__surface::-webkit-scrollbar-track {
  background: transparent;
}

.schema-form__surface::-webkit-scrollbar-thumb {
  background: var(--border-default);
  border-radius: 999px;
}

.schema-form__fallback-header {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.schema-form__fallback-header h3 {
  font-size: var(--text-body-sm);
  font-weight: var(--weight-title);
}

.schema-form__fallback-header p,
.schema-form__fallback-error {
  color: var(--text-secondary);
  font-size: var(--text-caption);
}

.schema-form__fallback-error {
  color: var(--text-error);
}

.schema-form__fallback-textarea {
  min-height: 16rem;
  font-family: var(--font-code);
}

.schema-form :deep(.schema-form__vertical-layout),
.schema-form :deep(.schema-form__group-item),
.schema-form :deep(.schema-form__array),
.schema-form :deep(.schema-form__array-item),
.schema-form :deep(.schema-form__array-item-content),
.schema-form :deep(.schema-form__one-of),
.schema-form :deep(.schema-form__categorization-panel) {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.schema-form :deep(.schema-form__horizontal-layout) {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-4);
}

.schema-form :deep(.schema-form__layout-item),
.schema-form :deep(.schema-form__control),
.schema-form :deep(.schema-form__control-wrapper) {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.schema-form :deep(.schema-form__label),
.schema-form :deep(.schema-form__group-label),
.schema-form :deep(.schema-form__array-legend),
.schema-form :deep(.schema-form__array-label),
.schema-form :deep(.schema-form__array-item-label),
.schema-form :deep(.schema-form__static-label),
.schema-form :deep(.schema-form__dialog-title) {
  color: var(--text-secondary);
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
}

.schema-form :deep(.schema-form__description) {
  color: var(--text-secondary);
  font-size: var(--text-tiny);
}

.schema-form :deep(input:not([type='checkbox']):not([type='radio'])),
.schema-form :deep(select),
.schema-form :deep(textarea) {
  background-color: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: var(--font-body);
  padding: var(--space-2) var(--space-3);
  width: 100%;
}

.schema-form :deep(select option) {
  background-color: var(--bg-panel);
  color: var(--text-primary);
}

.schema-form :deep(textarea) {
  min-height: 8rem;
}

.schema-form :deep(input::placeholder),
.schema-form :deep(textarea::placeholder) {
  color: var(--text-muted);
}

.schema-form :deep(input:not([type='checkbox']):not([type='radio']):focus-visible),
.schema-form :deep(select:focus-visible),
.schema-form :deep(textarea:focus-visible) {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
  border-color: var(--border-focus);
}

.schema-form :deep(input[type='checkbox']),
.schema-form :deep(input[type='radio']) {
  inline-size: 1rem;
  block-size: 1rem;
  accent-color: var(--border-focus);
  flex: 0 0 auto;
}

.schema-form :deep(fieldset) {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: color-mix(in srgb, var(--bg-panel) 92%, transparent);
}

.schema-form :deep(legend) {
  color: var(--text-secondary);
  padding: 0 var(--space-1);
}

.schema-form :deep(.schema-form__error) {
  color: var(--text-error);
  font-size: var(--text-tiny);
}

.schema-form :deep(.schema-form__asterisk),
.schema-form :deep(.schema-form__required) {
  color: var(--border-focus);
}

.schema-form :deep(.schema-form__group),
.schema-form :deep(.schema-form__array-item-wrapper),
.schema-form :deep(.schema-form__dialog) {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.schema-form :deep(.schema-form__array-toolbar),
.schema-form :deep(.schema-form__dialog-actions),
.schema-form :deep(.schema-form__stepper-footer) {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.schema-form :deep(.schema-form__empty) {
  color: var(--text-muted);
  font-size: var(--text-caption);
}

.schema-form :deep(.schema-form__categorization) {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.schema-form :deep(.schema-form__stepper) {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.schema-form :deep(.schema-form__stepper-badge) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  min-height: 1.5rem;
  border-radius: 999px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--text-tiny);
}

.schema-form :deep(.schema-form__stepper-line) {
  flex: 1;
  min-width: 2rem;
  border-top: 1px solid var(--border-default);
}

.schema-form :deep(.schema-form__category--selected) {
  border-color: var(--border-focus);
  color: var(--text-primary);
}
</style>
