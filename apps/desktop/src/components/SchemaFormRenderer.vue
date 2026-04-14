<script setup lang="ts">
import type { JsonSchema } from '@jsonforms/core';
import { JsonForms } from '@jsonforms/vue';
import { frozenRenderers } from '../lib/jsonforms-theme';
import { schemaFormAjv } from '../lib/schema-form';

const props = defineProps<{
  ariaLabel?: string;
  'aria-label'?: string;
  data: Record<string, unknown>;
  schema?: JsonSchema;
}>();

defineEmits<{
  change: [event: { data: unknown; errors: unknown[] }];
}>();
</script>

<template>
  <div class="schema-form-renderer" role="group" :aria-label="props.ariaLabel ?? props['aria-label']">
    <json-forms
      :ajv="schemaFormAjv"
      :data="data"
      :renderers="frozenRenderers"
      :schema="schema"
      validation-mode="ValidateAndShow"
      @change="$emit('change', $event)"
    />
  </div>
</template>

<style scoped>
.schema-form-renderer {
  max-height: min(32rem, 60vh);
  overflow-y: auto;
  padding-right: var(--space-1);
  scrollbar-width: thin;
  scrollbar-color: var(--border) transparent;
}

.schema-form-renderer::-webkit-scrollbar {
  width: 8px;
}

.schema-form-renderer::-webkit-scrollbar-track {
  background: transparent;
}

.schema-form-renderer::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 999px;
}

.schema-form-renderer :deep(.schema-form__vertical-layout),
.schema-form-renderer :deep(.schema-form__group-item),
.schema-form-renderer :deep(.schema-form__array),
.schema-form-renderer :deep(.schema-form__array-item),
.schema-form-renderer :deep(.schema-form__array-item-content),
.schema-form-renderer :deep(.schema-form__one-of),
.schema-form-renderer :deep(.schema-form__categorization-panel) {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.schema-form-renderer :deep(.schema-form__horizontal-layout) {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-4);
}

.schema-form-renderer :deep(.schema-form__layout-item),
.schema-form-renderer :deep(.schema-form__control),
.schema-form-renderer :deep(.schema-form__control-wrapper) {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.schema-form-renderer :deep(.schema-form__label),
.schema-form-renderer :deep(.schema-form__group-label),
.schema-form-renderer :deep(.schema-form__array-legend),
.schema-form-renderer :deep(.schema-form__array-label),
.schema-form-renderer :deep(.schema-form__array-item-label),
.schema-form-renderer :deep(.schema-form__static-label),
.schema-form-renderer :deep(.schema-form__dialog-title) {
  color: var(--muted-foreground);
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
}

.schema-form-renderer :deep(.schema-form__description) {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}

.schema-form-renderer :deep(input:not([type='checkbox']):not([type='radio'])),
.schema-form-renderer :deep(select),
.schema-form-renderer :deep(textarea) {
  background-color: var(--input);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--foreground);
  font-family: var(--font-body);
  padding: var(--space-2) var(--space-3);
  width: 100%;
}

.schema-form-renderer :deep(select option) {
  background-color: var(--card);
  color: var(--foreground);
}

.schema-form-renderer :deep(textarea) {
  min-height: 8rem;
}

.schema-form-renderer :deep(input::placeholder),
.schema-form-renderer :deep(textarea::placeholder) {
  color: var(--muted-foreground);
}

.schema-form-renderer :deep(input:not([type='checkbox']):not([type='radio']):focus-visible),
.schema-form-renderer :deep(select:focus-visible),
.schema-form-renderer :deep(textarea:focus-visible) {
  outline: 2px solid var(--ring);
  outline-offset: 2px;
  border-color: var(--ring);
}

.schema-form-renderer :deep(input[type='checkbox']),
.schema-form-renderer :deep(input[type='radio']) {
  inline-size: 1rem;
  block-size: 1rem;
  accent-color: var(--ring);
  flex: 0 0 auto;
}

.schema-form-renderer :deep(fieldset) {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: color-mix(in srgb, var(--card) 92%, transparent);
}

.schema-form-renderer :deep(legend) {
  color: var(--muted-foreground);
  padding: 0 var(--space-1);
}

.schema-form-renderer :deep(.schema-form__error) {
  color: var(--destructive);
  font-size: var(--text-tiny);
}

.schema-form-renderer :deep(.schema-form__asterisk),
.schema-form-renderer :deep(.schema-form__required) {
  color: var(--ring);
}

.schema-form-renderer :deep(.schema-form__group),
.schema-form-renderer :deep(.schema-form__array-item-wrapper),
.schema-form-renderer :deep(.schema-form__dialog) {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.schema-form-renderer :deep(.schema-form__array-toolbar),
.schema-form-renderer :deep(.schema-form__dialog-actions),
.schema-form-renderer :deep(.schema-form__stepper-footer) {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.schema-form-renderer :deep(.schema-form__empty) {
  color: var(--muted-foreground);
  font-size: var(--text-caption);
}

.schema-form-renderer :deep(.schema-form__categorization) {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.schema-form-renderer :deep(.schema-form__stepper) {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.schema-form-renderer :deep(.schema-form__stepper-badge) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  min-height: 1.5rem;
  border-radius: 999px;
  background: var(--muted);
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}

.schema-form-renderer :deep(.schema-form__stepper-line) {
  flex: 1;
  min-width: 2rem;
  border-top: 1px solid var(--border);
}

.schema-form-renderer :deep(.schema-form__category--selected) {
  border-color: var(--ring);
  color: var(--foreground);
}
</style>
