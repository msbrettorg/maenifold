<script setup lang="ts">
import { computed, provide, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { themedStyles } from '../lib/jsonforms-theme';
import {
  cloneRecord,
  getFormValidationErrors,
  inspectFormSchema,
  isRecord,
  normalizeFormData,
  stringifyRecord,
  validateFormData,
} from '../lib/schema-form';
import SchemaFormRawEditor from './SchemaFormRawEditor.vue';
import SchemaFormRenderer from './SchemaFormRenderer.vue';

const props = withDefaults(defineProps<{
  schema: unknown;
  data: Record<string, unknown>;
  ariaLabel?: string;
}>(), {
  ariaLabel: 'Schema form',
});

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

    if (nextSerialized !== rawJson.value) {
      rawJson.value = nextSerialized;
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

function handleRawInput(value: string): void {
  rawJson.value = value;

  try {
    const parsed = JSON.parse(value) as unknown;
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
  if (canToggleAdvancedRaw.value) {
    showAdvancedRaw.value = !showAdvancedRaw.value;
  }
}

function emitFormValidation(): void {
  emit('validation', {
    valid: validateFormData(formValidator.value, normalizeFormData(props.data)),
  });
}

function emitRawValidation(data: Record<string, unknown>): boolean {
  const errors = getFormValidationErrors(formValidator.value, data);

  if (errors == null) {
    rawJsonError.value = '';
    emit('validation', { valid: true });
    return true;
  }

  const valid = errors.length === 0;
  rawJsonError.value = valid ? '' : formatValidationError(errors[0]);
  emit('validation', { valid });
  return valid;
}

function formatValidationError(error: { instancePath?: string; message?: string }): string {
  const location = error.instancePath?.trim() ? error.instancePath : '/';
  const message = error.message?.trim() || 'Schema validation failed.';
  return `${location}: ${message}`;
}
</script>

<template>
  <section class="schema-form" role="form" :aria-label="props.ariaLabel">
    <div class="schema-form__surface">
      <div v-if="canToggleAdvancedRaw" class="schema-form__toolbar">
        <Button type="button" variant="ghost" @click="toggleAdvancedRaw">
          {{ showAdvancedRaw ? 'Back to Form' : 'Advanced JSON' }}
        </Button>
      </div>
      <SchemaFormRawEditor
        v-if="showRawEditor"
        :ariaLabel="`${props.ariaLabel} raw editor`"
        :message="rawEditorMessage"
        :title="rawEditorTitle"
        :value="rawJson"
        :error="rawJsonError"
        @input="handleRawInput"
      />
      <SchemaFormRenderer
        v-else
        :ariaLabel="`${props.ariaLabel} fields`"
        :data="props.data"
        :schema="formSchema ?? undefined"
        @change="handleFormChange"
      />
    </div>
  </section>
</template>

<style scoped>
.schema-form,
.schema-form__surface {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.schema-form__toolbar {
  display: flex;
  justify-content: flex-end;
}
</style>
