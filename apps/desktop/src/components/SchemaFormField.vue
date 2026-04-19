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
  <div class="schema-form__field">
    <template v-if="field.kind === 'boolean'">
      <div class="schema-form__switch-row">
        <Switch
          :id="fieldId()"
          :model-value="Boolean(value)"
          @update:model-value="emit('update', $event)"
        />
        <Label :for="fieldId()" class="schema-form__switch-label">
          {{ field.label }}
        </Label>
        <span v-if="field.required" class="schema-form__required">*</span>
      </div>
    </template>
    <template v-else>
      <div class="schema-form__label-row">
        <Label :for="fieldId()" class="schema-form__label">
          {{ field.label }}
        </Label>
        <span v-if="field.required" class="schema-form__required">*</span>
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

    <p v-if="field.description" class="schema-form__hint">
      {{ field.description }}
    </p>
    <p v-if="error" class="schema-form__error">
      {{ error }}
    </p>
  </div>
</template>
