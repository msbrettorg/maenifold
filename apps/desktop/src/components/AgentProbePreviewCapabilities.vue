<script setup lang="ts">
import type { SessionConfigSelectGroup, SessionConfigSelectOption, SessionConfigSelectOptions } from '../lib/types';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

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
const emit = defineEmits<{ 'update-field': [key: string, value: string] }>();

function isGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup,
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

</script>

<template>
  <div
    class="field-grid grid grid-cols-[repeat(auto-fit,minmax(12rem,1fr))] gap-[0.65rem]"
    :class="{ compact: props.compact }"
  >
    <div v-for="field in fields" :key="field.key" class="field-card grid gap-[0.35rem] min-w-0 p-0">
      <Label
        class="field-label text-[length:var(--text-caption)] font-normal text-[var(--text-secondary)]"
        :for="`preview-${field.key}`"
      >{{ field.label }}</Label>
      <Select
        :model-value="field.currentValue ?? EMPTY_VALUE"
        @update:model-value="(val) => emit('update-field', field.key, String(val))"
      >
        <SelectTrigger
          :id="`preview-${field.key}`"
          class="preview-select w-full px-[0.55rem] py-[0.45rem] border-none rounded-[4px] bg-[var(--surface-container)] text-[var(--color-text-primary)] text-[length:var(--text-caption)]"
        >
          <SelectValue :placeholder="field.emptyLabel" />
        </SelectTrigger>
        <SelectContent>
          <template v-for="entry in field.options" :key="isGroup(entry) ? entry.group : entry.value">
            <SelectGroup v-if="isGroup(entry)">
              <SelectLabel>{{ entry.name }}</SelectLabel>
              <SelectItem
                v-for="groupOption in entry.options"
                :key="groupOption.value"
                :value="groupOption.value"
              >
                {{ groupOption.name }}
              </SelectItem>
            </SelectGroup>
            <SelectItem v-else :value="entry.value">
              {{ entry.name }}
            </SelectItem>
          </template>
        </SelectContent>
      </Select>
    </div>
  </div>
</template>
