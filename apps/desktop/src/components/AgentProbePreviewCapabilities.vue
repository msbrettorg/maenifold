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

</script>

<template>
  <div class="field-grid" :class="{ compact: props.compact }">
    <div v-for="field in fields" :key="field.key" class="field-card">
      <label class="field-label" :for="`preview-${field.key}`">{{ field.label }}</label>
      <select :id="`preview-${field.key}`" class="preview-select" :value="field.currentValue ?? EMPTY_VALUE" disabled>
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
  padding: 0;
}

.field-label {
  font-size: var(--text-caption);
  font-weight: 400;
  color: #707070;
}

.preview-select {
  width: 100%;
  padding: 0.45rem 0.55rem;
  border: none;
  border-radius: 4px;
  background: #201f1f;
  color: var(--foreground);
  font-size: var(--text-caption);
  opacity: 0.75;
}
</style>
