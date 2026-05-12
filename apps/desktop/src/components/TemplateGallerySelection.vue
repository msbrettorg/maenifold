<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import type {
  PipelineTemplate,
  TemplateInputDefinition,
  TemplateInputValues,
} from '../lib/pipeline-templates';

const props = defineProps<{
  canCreate: boolean;
  hasIntentAgent: boolean;
  template: PipelineTemplate;
}>();

const emit = defineEmits<{
  (e: 'create', value: TemplateInputValues): void;
}>();

const inputDrafts = ref<TemplateInputValues>({});
const missingRequiredKeys = computed(
  () => new Set(props.template.runInputs
    .filter((input) => input.required && !inputDrafts.value[input.key]?.trim())
    .map((input) => input.key)),
);
const createDisabled = computed(
  () => !props.canCreate || !props.hasIntentAgent || missingRequiredKeys.value.size > 0,
);

watch(
  () => props.template.id,
  () => {
    inputDrafts.value = Object.fromEntries(
      props.template.runInputs.map((input) => [input.key, '']),
    );
  },
  { immediate: true },
);

function setInputValue(key: string, value: string | number): void {
  inputDrafts.value = { ...inputDrafts.value, [key]: String(value) };
}

function usesTextarea(input: TemplateInputDefinition): boolean {
  return input.type === 'textarea'
    || input.placeholder?.includes('\n') === true
    || input.description?.includes('\n') === true;
}

function fieldId(input: TemplateInputDefinition): string {
  return `template-input-${props.template.id}-${input.key}`;
}

function descriptionId(input: TemplateInputDefinition): string {
  return `${fieldId(input)}-description`;
}

function errorId(input: TemplateInputDefinition): string {
  return `${fieldId(input)}-error`;
}

function hasMissingRequired(input: TemplateInputDefinition): boolean {
  return missingRequiredKeys.value.has(input.key);
}

function describedBy(input: TemplateInputDefinition): string | undefined {
  const ids = [
    input.description ? descriptionId(input) : '',
    hasMissingRequired(input) ? errorId(input) : '',
  ].filter(Boolean);
  return ids.length > 0 ? ids.join(' ') : undefined;
}

function submitCreate(): void {
  if (createDisabled.value) return;
  emit('create', { ...inputDrafts.value });
}
</script>

<template>
  <form
    class="flex flex-col gap-3 p-4 w-[min(100%,720px)] border border-border-subtle rounded-[0.25rem] bg-surface"
    aria-label="Create template pipeline"
    @submit.prevent="submitCreate"
  >
    <div v-if="template.runInputs.length > 0" class="flex flex-col gap-3">
      <span class="text-[0.75rem] font-normal text-text-secondary">Run-time inputs</span>
      <div
        v-for="input in template.runInputs"
        :key="input.key"
        class="flex flex-col gap-1.5"
      >
        <Label :for="fieldId(input)" class="text-[0.875rem] font-medium text-text-primary">
          {{ input.label }}
          <span v-if="input.required" class="text-danger">*</span>
        </Label>
        <Textarea
          v-if="usesTextarea(input)"
          :id="fieldId(input)"
          class="min-h-20"
          :model-value="inputDrafts[input.key] ?? ''"
          :placeholder="input.placeholder"
          :aria-describedby="describedBy(input)"
          :aria-invalid="hasMissingRequired(input) ? 'true' : undefined"
          @update:model-value="(value) => setInputValue(input.key, value)"
        />
        <Input
          v-else
          :id="fieldId(input)"
          type="text"
          :model-value="inputDrafts[input.key] ?? ''"
          :placeholder="input.placeholder"
          :aria-describedby="describedBy(input)"
          :aria-invalid="hasMissingRequired(input) ? 'true' : undefined"
          @update:model-value="(value) => setInputValue(input.key, value)"
        />
        <p
          v-if="input.description"
          :id="descriptionId(input)"
          class="m-0 text-[0.75rem] text-text-secondary"
        >
          {{ input.description }}
        </p>
        <p
          v-if="hasMissingRequired(input)"
          :id="errorId(input)"
          class="m-0 text-[0.75rem] text-danger"
          role="alert"
        >
          Enter {{ input.label.toLowerCase() }} before creating this pipeline.
        </p>
      </div>
      <p class="m-0 text-text-secondary text-[0.75rem]">
        These values resolve template placeholders before the DAG is created.
      </p>
    </div>

    <div class="flex items-center justify-center gap-3">
      <Button type="submit" :disabled="createDisabled">
        Create pipeline
      </Button>
    </div>
  </form>
</template>
