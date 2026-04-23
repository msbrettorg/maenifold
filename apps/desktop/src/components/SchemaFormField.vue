<script setup lang="ts">
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
import type { SimpleField } from '../lib/schema-form-simple';

const props = defineProps<{
  field: SimpleField;
  value: unknown;
  error?: string;
}>();

const emit = defineEmits<{
  (e: 'update', next: unknown): void;
}>();

function fieldId(): string {
  return `schema-form-${props.field.key}`;
}

function numericModelValue(): string | number {
  return typeof props.value === 'number' ? props.value : '';
}

function stringModelValue(): string {
  return String(props.value ?? '');
}
</script>

<template>
  <div class="grid gap-1.5 rounded-[4px] p-3.5 bg-[color-mix(in_srgb,var(--color-bg-elevated)_88%,transparent)]">
    <template v-if="field.kind === 'boolean'">
      <div class="flex items-center gap-2">
        <Switch
          :id="fieldId()"
          :model-value="Boolean(value)"
          @update:model-value="emit('update', $event)"
        />
        <Label :for="fieldId()" class="text-text-primary text-sm font-[var(--weight-body)]">
          {{ field.label }}
        </Label>
        <span v-if="field.required" class="text-brand text-xs">*</span>
      </div>
    </template>
    <template v-else>
      <div class="flex items-center gap-2">
        <Label :for="fieldId()" class="text-text-primary text-sm font-[var(--weight-title)]">
          {{ field.label }}
        </Label>
        <span v-if="field.required" class="text-brand text-xs">*</span>
      </div>

      <Input
        v-if="field.kind === 'string'"
        :id="fieldId()"
        :model-value="stringModelValue()"
        type="text"
        @update:model-value="emit('update', $event)"
      />
      <Select
        v-else-if="field.kind === 'enum'"
        :model-value="stringModelValue()"
        @update:model-value="emit('update', $event)"
      >
        <SelectTrigger class="w-full rounded-[4px]">
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
        :id="fieldId()"
        :model-value="stringModelValue()"
        rows="5"
        @update:model-value="emit('update', $event)"
      />
      <Input
        v-else
        :id="fieldId()"
        :max="field.maximum"
        :min="field.minimum"
        :model-value="numericModelValue()"
        type="number"
        @update:model-value="emit('update', $event)"
      />
    </template>

    <p v-if="field.description" class="m-0 text-text-secondary text-xs">
      {{ field.description }}
    </p>
    <p v-if="error" class="m-0 text-danger text-xs">
      {{ error }}
    </p>
  </div>
</template>
