<script setup lang="ts">
import { computed, provide, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
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

interface FieldSchema {
  type?: string | string[];
  enum?: unknown[];
  title?: string;
  description?: string;
  format?: string;
  minimum?: number;
  maximum?: number;
}

interface SimpleObjectSchema extends Record<string, unknown> {
  type?: string | string[];
  properties: Record<string, FieldSchema>;
  required?: string[];
}

type SimpleFieldKind =
  | 'string'
  | 'enum'
  | 'textarea'
  | 'boolean'
  | 'integer'
  | 'number';

interface SimpleField {
  key: string;
  label: string;
  description?: string;
  kind: SimpleFieldKind;
  maximum?: number;
  minimum?: number;
  options?: Array<{ label: string; value: string }>;
  required: boolean;
}

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
const simpleFields = computed<SimpleField[]>(() => {
  const schema = simpleSchema.value;
  if (!schema) {
    return [];
  }

  const requiredFields = new Set(schema.required ?? []);
  return Object.entries(schema.properties).map(([key, property]) => {
    const resolvedType = getSchemaType(property);
    const kind = resolveFieldKind(property, resolvedType);
    const options = Array.isArray(property.enum)
      ? property.enum.map((value) => ({
          label: stringifyEnumLabel(value),
          value: String(value),
        }))
      : undefined;

    return {
      key,
      label: property.title?.trim() || sentenceCase(key),
      description:
        typeof property.description === 'string' && property.description.trim().length > 0
          ? property.description
          : undefined,
      kind: kind ?? 'string',
      maximum: typeof property.maximum === 'number' ? property.maximum : undefined,
      minimum: typeof property.minimum === 'number' ? property.minimum : undefined,
      options,
      required: requiredFields.has(key),
    };
  });
});
const simpleErrors = computed<Record<string, string>>(() => {
  const errors: Record<string, string> = {};

  for (const field of simpleFields.value) {
    const value = simpleValue.value[field.key];

    if (field.required && isBlankValue(value)) {
      errors[field.key] = `${field.label} is required.`;
      continue;
    }

    if ((field.kind === 'integer' || field.kind === 'number') && value !== undefined && value !== null && value !== '') {
      if (typeof value !== 'number' || !Number.isFinite(value)) {
        errors[field.key] = `${field.label} must be a valid number.`;
        continue;
      }

      if (field.kind === 'integer' && !Number.isInteger(value)) {
        errors[field.key] = `${field.label} must be a whole number.`;
        continue;
      }

      if (field.minimum !== undefined && value < field.minimum) {
        errors[field.key] = `${field.label} must be ${field.minimum} or greater.`;
        continue;
      }

      if (field.maximum !== undefined && value > field.maximum) {
        errors[field.key] = `${field.label} must be ${field.maximum} or less.`;
      }
    }
  }

  return errors;
});
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

function formatValidationError(error: { instancePath?: string; message?: string }): string {
  const location = error.instancePath?.trim() ? error.instancePath : '/';
  const message = error.message?.trim() || 'Schema validation failed.';
  return `${location}: ${message}`;
}

function toSimpleObjectSchema(schema: unknown): SimpleObjectSchema | null {
  if (!isRecord(schema) || !isRecord(schema.properties)) {
    return null;
  }

  const resolvedType = getSchemaType(schema as FieldSchema);
  if (resolvedType !== 'object') {
    return null;
  }

  const properties = Object.entries(schema.properties).reduce<Record<string, FieldSchema>>(
    (accumulator, [key, value]) => {
      if (!isRecord(value)) {
        return accumulator;
      }
      accumulator[key] = value as FieldSchema;
      return accumulator;
    },
    {},
  );

  if (Object.keys(properties).length === 0) {
    return null;
  }

  const allPropertiesSupported = Object.values(properties).every((property) => {
    const propertyType = getSchemaType(property);
    return resolveFieldKind(property, propertyType) !== null;
  });

  if (!allPropertiesSupported) {
    return null;
  }

  return {
    ...schema,
    properties,
    required: Array.isArray(schema.required)
      ? schema.required.filter((entry): entry is string => typeof entry === 'string')
      : [],
  } as SimpleObjectSchema;
}

function resolveFieldKind(
  property: FieldSchema,
  propertyType: string | null,
): SimpleFieldKind | null {
  if (propertyType === 'string' && Array.isArray(property.enum) && property.enum.every((entry) => typeof entry === 'string')) {
    return 'enum';
  }

  if (propertyType === 'string' && property.format === 'textarea') {
    return 'textarea';
  }

  if (propertyType === 'string') {
    return 'string';
  }

  if (propertyType === 'boolean') {
    return 'boolean';
  }

  if (propertyType === 'integer' || propertyType === 'number') {
    return propertyType;
  }

  return null;
}

function getSchemaType(property: FieldSchema): string | null {
  if (typeof property.type === 'string') {
    return property.type;
  }

  if (Array.isArray(property.type)) {
    return property.type.find((entry): entry is string => typeof entry === 'string') ?? null;
  }

  return null;
}

function normalizeSimpleFieldValue(field: SimpleField, value: unknown): unknown {
  if (field.kind === 'boolean') {
    return value === true;
  }

  if (field.kind === 'integer' || field.kind === 'number') {
    const raw = typeof value === 'number' ? String(value) : String(value ?? '').trim();
    if (!raw) {
      return undefined;
    }

    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      return undefined;
    }

    return field.kind === 'integer' ? Math.trunc(parsed) : parsed;
  }

  return typeof value === 'string' ? value : String(value ?? '');
}

function stringifyEnumLabel(value: unknown): string {
  if (value === '__none__') {
    return 'None';
  }
  return typeof value === 'string' && value.trim().length > 0 ? value : String(value ?? '');
}

function sentenceCase(value: string): string {
  return value
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^./, (character) => character.toUpperCase());
}

function isBlankValue(value: unknown): boolean {
  return value == null || (typeof value === 'string' && value.trim().length === 0);
}

function fieldId(fieldKey: string): string {
  return `schema-form-${fieldKey}`;
}

function numericFieldModelValue(fieldKey: string): string | number {
  const value = simpleValue.value[fieldKey];
  return typeof value === 'number' ? value : '';
}
</script>

<template>
  <section class="schema-form" role="form" :aria-label="props.ariaLabel">
    <div class="schema-form__surface">
      <div v-if="isSimpleMode" class="schema-form__fields">
        <div
          v-for="field in simpleFields"
          :key="field.key"
          class="schema-form__field"
        >
          <template v-if="field.kind === 'boolean'">
            <div class="schema-form__switch-row">
              <Switch
                :id="fieldId(field.key)"
                :model-value="Boolean(simpleValue[field.key])"
                @update:model-value="updateSimpleField(field, $event)"
              />
              <Label :for="fieldId(field.key)" class="schema-form__switch-label">
                {{ field.label }}
              </Label>
              <span v-if="field.required" class="schema-form__required">*</span>
            </div>
          </template>
          <template v-else>
            <div class="schema-form__label-row">
              <Label :for="fieldId(field.key)" class="schema-form__label">
                {{ field.label }}
              </Label>
              <span v-if="field.required" class="schema-form__required">*</span>
            </div>

            <Input
              v-if="field.kind === 'string'"
              :id="fieldId(field.key)"
              :model-value="String(simpleValue[field.key] ?? '')"
              type="text"
              @update:model-value="updateSimpleField(field, $event)"
            />
            <Select
              v-else-if="field.kind === 'enum'"
              :model-value="String(simpleValue[field.key] ?? '')"
              @update:model-value="updateSimpleField(field, $event)"
            >
              <SelectTrigger class="schema-form__select-trigger">
                <SelectValue :placeholder="`Select ${field.label.toLowerCase()}`" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in field.options ?? []"
                  :key="`${field.key}-${option.value}`"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Textarea
              v-else-if="field.kind === 'textarea'"
              :id="fieldId(field.key)"
              :model-value="String(simpleValue[field.key] ?? '')"
              rows="5"
              @update:model-value="updateSimpleField(field, $event)"
            />
            <Input
              v-else
              :id="fieldId(field.key)"
              :max="field.maximum"
              :min="field.minimum"
              :model-value="numericFieldModelValue(field.key)"
              type="number"
              @update:model-value="updateSimpleField(field, $event)"
            />
          </template>

          <p v-if="field.description" class="schema-form__hint">
            {{ field.description }}
          </p>
          <p v-if="simpleErrors[field.key]" class="schema-form__error">
            {{ simpleErrors[field.key] }}
          </p>
        </div>
      </div>

      <template v-else>
        <div v-if="canToggleAdvancedRaw" class="schema-form__toolbar">
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

.schema-form__fields {
  display: grid;
  gap: 0.875rem;
}

.schema-form__field {
  display: grid;
  gap: 0.375rem;
  padding: 0.875rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-bg-elevated) 88%, transparent);
}

.schema-form__label-row,
.schema-form__switch-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.schema-form__label,
.schema-form__switch-label {
  color: var(--color-text-primary);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
}

.schema-form__switch-label {
  font-weight: var(--weight-body);
}

.schema-form__select-trigger {
  width: 100%;
  border-radius: 4px;
}

.schema-form__required {
  color: var(--color-brand);
  font-size: var(--text-xs);
}

.schema-form__hint {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: var(--text-xs);
}

.schema-form__error {
  margin: 0;
  color: var(--color-danger);
  font-size: var(--text-xs);
}
</style>
