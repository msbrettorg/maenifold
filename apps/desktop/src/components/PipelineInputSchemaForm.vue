<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

type SchemaProperty = {
  type?: string;
  enum?: unknown[];
  title?: string;
};

type ObjectSchema = {
  properties?: Record<string, SchemaProperty>;
  required?: string[];
};

const props = defineProps<{
  schema: ObjectSchema;
  modelValue: Record<string, unknown>;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: Record<string, unknown>): void;
  (e: 'submit', value: Record<string, unknown>): void;
}>();

const errors = ref<Record<string, string>>({});
const properties = computed(() => Object.entries(props.schema.properties ?? {}));
const requiredFields = computed(() => new Set(props.schema.required ?? []));
const draft = ref<Record<string, unknown>>({ ...props.modelValue });

watch(
  () => props.modelValue,
  (value) => {
    draft.value = { ...value };
  },
  { deep: true },
);

function labelFor(name: string, property: SchemaProperty): string {
  return property.title ?? name;
}

function isEmpty(value: unknown): boolean {
  return value === undefined || value === null || value === '';
}

function updateField(name: string, property: SchemaProperty, value: unknown): void {
  const next = { ...draft.value };
  if (property.type === 'integer' || property.type === 'number') {
    const text = String(value);
    next[name] = text.trim() === '' ? '' : Number(text);
  } else if (property.type === 'boolean') {
    next[name] = value === true;
  } else {
    next[name] = value;
  }
  draft.value = next;
  emit('update:modelValue', { ...next });
}

function validateDraft(): boolean {
  const nextErrors: Record<string, string> = {};
  for (const [name] of properties.value) {
    if (requiredFields.value.has(name) && isEmpty(draft.value[name])) {
      nextErrors[name] = `${name} is required.`;
    }
  }
  errors.value = nextErrors;
  return Object.keys(nextErrors).length === 0;
}

function submitForm(): void {
  if (!validateDraft()) return;
  emit('submit', { ...draft.value });
}
</script>

<template>
  <form class="flex flex-col gap-3" aria-label="Pipeline inputs" @submit.prevent="submitForm">
    <div v-for="[name, property] in properties" :key="name" class="flex flex-col gap-1.5">
      <Label :for="`pipeline-input-${name}`" class="text-xs font-medium text-text-secondary">
        {{ labelFor(name, property) }}
      </Label>

      <Select
        v-if="property.type === 'string' && property.enum"
        :name="name"
        :model-value="String(draft[name] ?? '')"
        @update:model-value="updateField(name, property, $event)"
      >
        <SelectTrigger :id="`pipeline-input-${name}`" class="w-full">
          <SelectValue :placeholder="`Select ${name}`" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="option in property.enum"
            :key="String(option)"
            :value="String(option)"
          >
            {{ String(option) }}
          </SelectItem>
        </SelectContent>
      </Select>

      <Checkbox
        v-else-if="property.type === 'boolean'"
        :id="`pipeline-input-${name}`"
        :name="name"
        :checked="draft[name] === true"
        @update:checked="updateField(name, property, $event)"
      />

      <Input
        v-else
        :id="`pipeline-input-${name}`"
        :name="name"
        :type="property.type === 'integer' || property.type === 'number' ? 'number' : 'text'"
        :model-value="draft[name] == null ? '' : String(draft[name])"
        :aria-invalid="errors[name] ? 'true' : undefined"
        @update:model-value="updateField(name, property, $event)"
      />

      <p v-if="errors[name]" class="m-0 text-xs text-danger">{{ errors[name] }}</p>
    </div>

    <Button type="submit" class="w-fit">Run with inputs</Button>
  </form>
</template>
