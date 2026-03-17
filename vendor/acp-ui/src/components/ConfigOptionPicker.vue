<script setup lang="ts">
import type {
  SessionConfigOption,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
} from '../lib/types';

const props = defineProps<{
  option: SessionConfigOption;
}>();

const emit = defineEmits<{
  change: [configId: string, value: string];
}>();

function isGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

function handleChange(event: Event) {
  const target = event.target as HTMLSelectElement;
  emit('change', props.option.id, target.value);
}
</script>

<template>
  <label
    class="config-option-picker"
    :title="option.description || option.name"
  >
    <span class="config-option-name">{{ option.name }}</span>
    <select
      class="config-option-select"
      :value="option.currentValue"
      @change="handleChange"
    >
      <template v-for="entry in option.options" :key="isGroup(entry) ? entry.group : entry.value">
        <optgroup v-if="isGroup(entry)" :label="entry.name">
          <option
            v-for="groupOption in entry.options"
            :key="groupOption.value"
            :value="groupOption.value"
            :title="groupOption.description"
          >
            {{ groupOption.name }}
          </option>
        </optgroup>
        <option
          v-else
          :value="entry.value"
          :title="entry.description"
        >
          {{ entry.name }}
        </option>
      </template>
    </select>
  </label>
</template>

<style scoped>
.config-option-picker {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.config-option-name {
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--text-secondary, #666);
  white-space: nowrap;
}

.config-option-select {
  min-width: 140px;
  max-width: 220px;
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--border-color, #ccc);
  border-radius: 6px;
  background: var(--bg-main, #fff);
  color: var(--text-primary, #222);
  font-size: 0.8rem;
}

.config-option-select:focus {
  outline: none;
  border-color: var(--text-accent, #0066cc);
}
</style>
