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
import {
  buildSimpleFields,
  computeSimpleErrors,
  formatValidationError,
  normalizeSimpleFieldValue,
  toSimpleObjectSchema,
  type SimpleField,
} from '../lib/schema-form-simple';
import SchemaFormField from './SchemaFormField.vue';
import SchemaFormRawEditor from './SchemaFormRawEditor.vue';
import SchemaFormRenderer from './SchemaFormRenderer.vue';

const EMPTY_RECORD: Readonly<Record<string, unknown>> = Object.freeze({});

const props = withDefaults(
  defineProps<{
    schema: unknown;
    modelValue?: Record<string, unknown>;
    data?: Record<string, unknown>;
    ariaLabel?: string;
  }>(),
  {
    ariaLabel: 'Schema form',
    modelValue: undefined,
    data: undefined,
  },
);

const emit = defineEmits<{
  change: [data: Record<string, unknown>];
  validation: [state: { valid: boolean }];
  'update:modelValue': [data: Record<string, unknown>];
}>();

const rawJson = ref('');
const rawJsonError = ref('');
const showAdvancedRaw = ref(false);
const lastPublishedData = ref('{}');
const simpleValue = ref<Record<string, unknown>>({});

provide('styles', themedStyles);

const resolvedData = computed(() => props.modelValue ?? props.data ?? EMPTY_RECORD);
const normalizedResolvedData = computed(() => normalizeFormData(resolvedData.value));

const simpleSchema = computed(() => toSimpleObjectSchema(props.schema));
const simpleFields = computed<SimpleField[]>(() => buildSimpleFields(simpleSchema.value));
const simpleErrors = computed<Record<string, string>>(() =>
  computeSimpleErrors(simpleFields.value, simpleValue.value),
);
const isSimpleMode = computed(() => simpleSchema.value !== null);

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
const rawEditorTitle = computed(() =>
  fallbackReason.value === null ? 'Advanced raw JSON' : 'Raw JSON',
);
const rawEditorMessage = computed(() =>
  fallbackReason.value === null
    ? 'Advanced mode bypasses the form renderer and edits the payload directly.'
    : fallbackMessage.value,
);
const canToggleAdvancedRaw = computed(() => fallbackReason.value === null);
const showRawEditor = computed(
  () => !isSimpleMode.value && (fallbackReason.value !== null || showAdvancedRaw.value),
);
const formSchema = computed(() => schemaAssessment.value.formSchema);
const formValidator = computed(() => schemaAssessment.value.validator);

watch(
  () => stringifyRecord(normalizedResolvedData.value),
  (serializedValue) => {
    lastPublishedData.value = serializedValue;
    simpleValue.value = JSON.parse(serializedValue) as Record<string, unknown>;

    if (isSimpleMode.value) {
      emitSimpleValidation();
      return;
    }

    if (!showRawEditor.value) {
      emitFormValidation();
      return;
    }

    if (serializedValue !== rawJson.value) {
      rawJson.value = serializedValue;
    }
    emitRawValidation(normalizedResolvedData.value);
  },
  { immediate: true },
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
  () => [props.schema, showRawEditor.value, isSimpleMode.value] as const,
  () => {
    if (isSimpleMode.value) {
      emitSimpleValidation();
      return;
    }

    if (showRawEditor.value) {
      rawJson.value = stringifyRecord(normalizedResolvedData.value);
      lastPublishedData.value = rawJson.value;
      emitRawValidation(normalizedResolvedData.value);
      return;
    }

    emitFormValidation();
  },
  { immediate: true },
);

watch(
  () => stringifyRecord(simpleValue.value),
  () => {
    if (isSimpleMode.value) {
      emitSimpleValidation();
    }
  },
);

function handleFormChange(event: { data: unknown; errors: unknown[] }): void {
  emit('validation', { valid: event.errors.length === 0 });
  publishValue(normalizeFormData(event.data));
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

    publishValue(nextData);
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

function updateSimpleField(field: SimpleField, nextValue: unknown): void {
  const nextData = {
    ...simpleValue.value,
    [field.key]: normalizeSimpleFieldValue(field, nextValue),
  };

  simpleValue.value = nextData;
  publishValue(nextData);
}

function emitSimpleValidation(): void {
  emit('validation', { valid: Object.keys(simpleErrors.value).length === 0 });
}

function emitFormValidation(): void {
  emit('validation', {
    valid: validateFormData(formValidator.value, normalizedResolvedData.value),
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

function publishValue(nextData: Record<string, unknown>): void {
  const nextSerialized = stringifyRecord(nextData);
  if (nextSerialized === lastPublishedData.value) {
    return;
  }

  lastPublishedData.value = nextSerialized;
  emit('update:modelValue', nextData);
  emit('change', nextData);
}
</script>

<template>
  <section class="flex flex-col gap-4" role="form" :aria-label="props.ariaLabel">
    <div class="flex flex-col gap-4">
      <div v-if="isSimpleMode" class="grid gap-3.5">
        <SchemaFormField
          v-for="field in simpleFields"
          :key="field.key"
          :field="field"
          :value="simpleValue[field.key]"
          :error="simpleErrors[field.key]"
          @update="updateSimpleField(field, $event)"
        />
      </div>

      <template v-else>
        <div v-if="canToggleAdvancedRaw" class="flex justify-end">
          <Button type="button" variant="ghost" @click="toggleAdvancedRaw">
            {{ showAdvancedRaw ? 'Back to form' : 'Advanced JSON' }}
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
          :data="normalizedResolvedData"
          :schema="formSchema ?? undefined"
          @change="handleFormChange"
        />
      </template>
    </div>
  </section>
</template>
