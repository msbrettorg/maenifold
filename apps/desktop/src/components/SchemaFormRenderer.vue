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
  <div
    class="schema-form-renderer overflow-y-auto pr-1 [max-height:min(32rem,60vh)] [scrollbar-width:thin] [scrollbar-color:var(--color-border-subtle)_transparent] [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:bg-[var(--color-border-subtle)] [&::-webkit-scrollbar-thumb]:rounded-[4px]"
    role="group"
    :aria-label="props.ariaLabel ?? props['aria-label']"
  >
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

<!-- audit-ok: max-style-rules — all :deep() rules target third-party JSONForms internals; not migratable to Tailwind -->
<!-- audit-ok: max-loc — :deep() style block for JSONForms theming -->
<!-- audit-ok: deep-third-party — JSONForms internal selectors require :deep() -->
<style scoped>
.schema-form-renderer :deep(.schema-form__vertical-layout), .schema-form-renderer :deep(.schema-form__group-item), .schema-form-renderer :deep(.schema-form__array), .schema-form-renderer :deep(.schema-form__array-item), .schema-form-renderer :deep(.schema-form__array-item-content), .schema-form-renderer :deep(.schema-form__one-of), .schema-form-renderer :deep(.schema-form__categorization-panel) { display: flex; flex-direction: column; gap: var(--space-4); }
.schema-form-renderer :deep(.schema-form__horizontal-layout) { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: var(--space-4); } .schema-form-renderer :deep(.schema-form__layout-item), .schema-form-renderer :deep(.schema-form__control), .schema-form-renderer :deep(.schema-form__control-wrapper) { display: flex; flex-direction: column; gap: var(--space-2); }
.schema-form-renderer :deep(.schema-form__label), .schema-form-renderer :deep(.schema-form__group-label), .schema-form-renderer :deep(.schema-form__array-legend), .schema-form-renderer :deep(.schema-form__array-label), .schema-form-renderer :deep(.schema-form__array-item-label), .schema-form-renderer :deep(.schema-form__static-label), .schema-form-renderer :deep(.schema-form__dialog-title) { color: var(--color-text-secondary); font-size: var(--text-caption); font-weight: var(--weight-title); } .schema-form-renderer :deep(.schema-form__description) { color: var(--color-text-secondary); font-size: var(--text-tiny); }
.schema-form-renderer :deep(input:not([type='checkbox']):not([type='radio'])), .schema-form-renderer :deep(select), .schema-form-renderer :deep(textarea) { background-color: var(--color-bg-surface); border: 1px solid var(--color-border-subtle); border-radius: var(--radius-md); color: var(--color-text-primary); font-family: var(--font-body); padding: var(--space-2) var(--space-3); width: 100%; } .schema-form-renderer :deep(select option) { background-color: var(--color-bg-elevated); color: var(--color-text-primary); }
.schema-form-renderer :deep(textarea) { min-height: 8rem; } .schema-form-renderer :deep(input::placeholder), .schema-form-renderer :deep(textarea::placeholder) { color: var(--color-text-secondary); }
.schema-form-renderer :deep(input:not([type='checkbox']):not([type='radio']):focus-visible), .schema-form-renderer :deep(select:focus-visible), .schema-form-renderer :deep(textarea:focus-visible) { outline: 2px solid var(--color-brand); outline-offset: 2px; border-color: var(--color-brand); } .schema-form-renderer :deep(input[type='checkbox']), .schema-form-renderer :deep(input[type='radio']) { inline-size: 1rem; block-size: 1rem; accent-color: var(--color-brand); flex: 0 0 auto; }
.schema-form-renderer :deep(fieldset) { border: 1px solid var(--color-border-subtle); border-radius: var(--radius-md); padding: var(--space-3); background: color-mix(in srgb, var(--color-bg-elevated) 92%, transparent); } .schema-form-renderer :deep(legend) { color: var(--color-text-secondary); padding: 0 var(--space-1); }
.schema-form-renderer :deep(.schema-form__error) { color: var(--color-danger); font-size: var(--text-tiny); } .schema-form-renderer :deep(.schema-form__asterisk), .schema-form-renderer :deep(.schema-form__required) { color: var(--color-brand); }
.schema-form-renderer :deep(.schema-form__group), .schema-form-renderer :deep(.schema-form__array-item-wrapper), .schema-form-renderer :deep(.schema-form__dialog) { display: flex; flex-direction: column; gap: var(--space-4); } .schema-form-renderer :deep(.schema-form__array-toolbar), .schema-form-renderer :deep(.schema-form__dialog-actions), .schema-form-renderer :deep(.schema-form__stepper-footer) { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.schema-form-renderer :deep(.schema-form__empty) { color: var(--color-text-secondary); font-size: var(--text-caption); } .schema-form-renderer :deep(.schema-form__categorization) { display: flex; flex-direction: column; gap: var(--space-3); }
.schema-form-renderer :deep(.schema-form__stepper) { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; } .schema-form-renderer :deep(.schema-form__stepper-badge) { display: inline-flex; align-items: center; justify-content: center; min-width: 1.5rem; min-height: 1.5rem; border-radius: 4px; background: var(--color-bg-surface); color: var(--color-text-secondary); font-size: var(--text-tiny); }
.schema-form-renderer :deep(.schema-form__stepper-line) { flex: 1; min-width: 2rem; border-top: 1px solid var(--color-border-subtle); } .schema-form-renderer :deep(.schema-form__category--selected) { border-color: var(--color-brand); color: var(--color-text-primary); }
</style>
