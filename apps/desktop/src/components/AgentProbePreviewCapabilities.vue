<script setup lang="ts">
import type { SessionConfigSelectGroup, SessionConfigSelectOption, SessionConfigSelectOptions } from '../lib/types';

const EMPTY_VALUE = '__preview-empty__';

interface PreviewField {
  key: string;
  label: string;
  description?: string;
  currentValue?: string;
  options: SessionConfigSelectOptions;
  emptyLabel: string;
}

const props = defineProps<{
  compact: boolean;
  fields: PreviewField[];
}>();

function isGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup,
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

function flattenOptions(options: SessionConfigSelectOptions): SessionConfigSelectOption[] {
  return options.flatMap((entry) => (isGroup(entry) ? entry.options : [entry]));
}

function optionCount(field: PreviewField): number {
  return flattenOptions(field.options).length;
}

function currentLabel(field: PreviewField): string {
  if (!field.currentValue) {
    return field.emptyLabel;
  }

  const match = flattenOptions(field.options).find((option) => option.value === field.currentValue);
  return match?.name ?? field.currentValue;
}
</script>

<template>
  <div class="field-grid" :class="{ compact: props.compact }">
    <div v-for="field in fields" :key="field.key" class="field-card">
      <div class="field-heading">
        <span class="field-label">{{ field.label }}</span>
        <span class="option-count">{{ optionCount(field) }} options</span>
      </div>

      <select class="preview-select" :value="field.currentValue ?? EMPTY_VALUE" disabled>
        <option v-if="!field.currentValue" :value="EMPTY_VALUE">{{ field.emptyLabel }}</option>
        <template v-for="entry in field.options" :key="isGroup(entry) ? entry.group : entry.value">
          <optgroup v-if="isGroup(entry)" :label="entry.name">
            <option
              v-for="groupOption in entry.options"
              :key="groupOption.value"
              :value="groupOption.value"
            >
              {{ groupOption.name }}
            </option>
          </optgroup>
          <option v-else :value="entry.value">
            {{ entry.name }}
          </option>
        </template>
      </select>

      <div class="field-current" :title="currentLabel(field)">{{ currentLabel(field) }}</div>
      <p v-if="field.description" class="field-description">{{ field.description }}</p>
    </div>
  </div>
</template>

<style scoped>
.field-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: 0.65rem;
}

.field-card {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  padding: 0.65rem;
  border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--card) 72%, transparent);
}

.field-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
}

.field-label,
.option-count,
.field-current,
.field-description {
  font-size: var(--text-caption);
}

.compact .field-label,
.compact .option-count {
  font-size: var(--text-tiny);
}

.option-count,
.field-description {
  color: var(--muted-foreground);
}

.preview-select {
  width: 100%;
  padding: 0.45rem 0.55rem;
  border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--input) 92%, transparent);
  color: var(--foreground);
  font-size: var(--text-caption);
  opacity: 0.75;
}

.field-current {
  min-width: 0;
  font-weight: var(--weight-display);
  color: var(--foreground);
  word-break: break-word;
}

.compact .field-current {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 640px) {
  .field-grid {
    grid-template-columns: 1fr;
  }

  .field-heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 0.25rem;
  }
}
</style>
