<script setup lang="ts">
import type { JsonSchema } from '@jsonforms/core';
import { JsonForms } from '@jsonforms/vue';
import { computed, provide, ref, watch } from 'vue';
import { frozenRenderers, themedStyles } from '../lib/jsonforms-theme';

type FallbackReason = 'missing' | 'unsupported' | 'complex';

interface SchemaComplexity {
  arrayCount: number;
  combinatorCount: number;
  enumValueCount: number;
  maxDepth: number;
  objectCount: number;
  propertyCount: number;
}

const EMPTY_RECORD: Readonly<Record<string, unknown>> = Object.freeze({});
const COMPLEX_SCHEMA_LIMITS = Object.freeze({
  arrayCount: 3,
  combinatorCount: 0,
  maxDepth: 4,
  objectCount: 7,
  propertyCount: 18,
  score: 28,
});

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
const lastPublishedData = ref(stringifyRecord(props.data));

provide('styles', themedStyles);

const fallbackReason = computed<FallbackReason | null>(() => getFallbackReason(props.schema));
const fallbackMessage = computed(() => {
  switch (fallbackReason.value) {
    case 'complex':
      return 'Schema is too large or deeply nested for the inspector — edit the payload directly.';
    case 'unsupported':
      return 'Schema shape is not supported in the inspector — edit the payload directly.';
    default:
      return 'Schema unavailable — edit the payload directly.';
  }
});
const showRawFallback = computed(() => fallbackReason.value !== null);
const formSchema = computed<JsonSchema>(() => props.schema as JsonSchema);

watch(
  () => props.data,
  (value) => {
    const nextSerialized = stringifyRecord(value);
    lastPublishedData.value = nextSerialized;
    if (!showRawFallback.value) {
      return;
    }

    const nextRawJson = nextSerialized;
    if (nextRawJson !== rawJson.value) {
      rawJson.value = nextRawJson;
    }
    rawJsonError.value = '';
    emit('validation', { valid: true });
  },
  { deep: true, immediate: true },
);

watch(
  () => props.schema,
  () => {
    if (showRawFallback.value) {
      rawJson.value = stringifyRecord(props.data);
      lastPublishedData.value = rawJson.value;
      rawJsonError.value = '';
      emit('validation', { valid: true });
    }
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

    rawJsonError.value = '';
    emit('validation', { valid: true });
    const nextData = cloneRecord(parsed);
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function cloneRecord<T extends Record<string, unknown>>(value: T): T {
  if (typeof structuredClone === 'function') {
    return structuredClone(value);
  }

  return JSON.parse(JSON.stringify(value)) as T;
}

function stringifyRecord(value: Record<string, unknown>): string {
  return JSON.stringify(value, null, 2);
}

function normalizeFormData(value: unknown): Record<string, unknown> {
  return isRecord(value) ? cloneRecord(value) : cloneRecord(EMPTY_RECORD);
}

function getFallbackReason(schema: unknown): FallbackReason | null {
  if (schema == null) {
    return 'missing';
  }
  if (!isSchemaObject(schema)) {
    return 'unsupported';
  }

  const complexity = analyzeSchema(schema);
  const score =
    complexity.propertyCount
    + complexity.objectCount * 2
    + complexity.arrayCount * 4
    + complexity.combinatorCount * 6
    + Math.max(0, complexity.maxDepth - 2) * 4
    + Math.ceil(complexity.enumValueCount / 10);

  if (
    complexity.combinatorCount > COMPLEX_SCHEMA_LIMITS.combinatorCount
    || complexity.maxDepth > COMPLEX_SCHEMA_LIMITS.maxDepth
    || complexity.arrayCount > COMPLEX_SCHEMA_LIMITS.arrayCount
    || complexity.objectCount > COMPLEX_SCHEMA_LIMITS.objectCount
    || complexity.propertyCount > COMPLEX_SCHEMA_LIMITS.propertyCount
    || score > COMPLEX_SCHEMA_LIMITS.score
  ) {
    return 'complex';
  }

  return null;
}

function analyzeSchema(schema: Record<string, unknown>, depth = 1, seen = new Set<object>()): SchemaComplexity {
  if (seen.has(schema)) {
    return {
      arrayCount: 0,
      combinatorCount: 0,
      enumValueCount: 0,
      maxDepth: depth,
      objectCount: 0,
      propertyCount: 0,
    };
  }
  seen.add(schema);

  const properties = isSchemaObject(schema.properties) ? Object.values(schema.properties) : [];
  const combinators = [
    ...collectSchemaArray(schema.oneOf),
    ...collectSchemaArray(schema.anyOf),
    ...collectSchemaArray(schema.allOf),
  ];
  const prefixedItems = collectSchemaArray(schema.prefixItems);
  const childSchemas = [
    ...properties.filter(isSchemaObject),
    ...combinators,
    ...prefixedItems,
    ...collectSingleSchema(schema.items),
    ...collectSingleSchema(schema.additionalProperties),
    ...collectSingleSchema(schema.contains),
    ...collectPropertySchemas(schema.$defs),
    ...collectPropertySchemas(schema.definitions),
  ];

  const stats: SchemaComplexity = {
    arrayCount: hasArrayShape(schema) ? 1 : 0,
    combinatorCount: combinators.length,
    enumValueCount: Array.isArray(schema.enum) ? schema.enum.length : 0,
    maxDepth: depth,
    objectCount: hasObjectShape(schema) ? 1 : 0,
    propertyCount: properties.length,
  };

  for (const child of childSchemas) {
    const childStats = analyzeSchema(child, depth + 1, seen);
    stats.arrayCount += childStats.arrayCount;
    stats.combinatorCount += childStats.combinatorCount;
    stats.enumValueCount += childStats.enumValueCount;
    stats.maxDepth = Math.max(stats.maxDepth, childStats.maxDepth);
    stats.objectCount += childStats.objectCount;
    stats.propertyCount += childStats.propertyCount;
  }

  return stats;
}

function isSchemaObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function hasObjectShape(schema: Record<string, unknown>): boolean {
  return schema.type === 'object' || isSchemaObject(schema.properties);
}

function hasArrayShape(schema: Record<string, unknown>): boolean {
  return schema.type === 'array' || schema.items != null || Array.isArray(schema.prefixItems);
}

function collectSchemaArray(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter(isSchemaObject) : [];
}

function collectSingleSchema(value: unknown): Record<string, unknown>[] {
  return isSchemaObject(value) ? [value] : [];
}

function collectPropertySchemas(value: unknown): Record<string, unknown>[] {
  return isSchemaObject(value) ? Object.values(value).filter(isSchemaObject) : [];
}
</script>

<template>
  <section class="schema-form">
    <div class="schema-form__surface">
      <div v-if="showRawFallback" class="schema-form__fallback panel">
        <div class="schema-form__fallback-header">
          <h3>Raw JSON</h3>
          <p>{{ fallbackMessage }}</p>
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
        :data="props.data"
        :renderers="frozenRenderers"
        :schema="formSchema"
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
  font-size: var(--text-base);
  font-weight: 600;
}

.schema-form__fallback-header p,
.schema-form__fallback-error {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.schema-form__fallback-error {
  color: var(--text-error);
}

.schema-form__fallback-textarea {
  min-height: 16rem;
  font-family: var(--font-mono);
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
  font-size: var(--text-sm);
  font-weight: 600;
}

.schema-form :deep(.schema-form__description) {
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.schema-form :deep(input:not([type='checkbox']):not([type='radio'])),
.schema-form :deep(select),
.schema-form :deep(textarea) {
  background-color: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: var(--font-sans);
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
  accent-color: var(--text-accent);
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
  font-size: var(--text-xs);
}

.schema-form :deep(.schema-form__asterisk),
.schema-form :deep(.schema-form__required) {
  color: var(--text-accent);
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
  font-size: var(--text-sm);
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
  font-size: var(--text-xs);
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
