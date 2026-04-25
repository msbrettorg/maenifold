<script setup lang="ts">
import { ref } from 'vue';
import { Input } from '@/components/ui/input';

const props = defineProps<{
  error: string;
  hasIntentAgent: boolean;
  modelValue: string;
  submitting: boolean;
}>();

const emit = defineEmits<{
  submit: [];
  'update:modelValue': [value: string];
}>();

const intentInputRef = ref<HTMLElement | { $el?: HTMLElement } | null>(null);

function focusIntentInput(): void {
  if (intentInputRef.value instanceof HTMLElement) {
    intentInputRef.value.focus();
    return;
  }
  const inputElement = intentInputRef.value?.$el instanceof HTMLElement
    ? intentInputRef.value.$el
    : document.getElementById('pipeline-intent-input');
  inputElement?.focus();
}

defineExpose({ focusIntentInput });
</script>

<template>
  <section class="w-full flex flex-col items-center justify-center pt-4 pb-1">
    <h3 class="text-lg font-semibold text-text-primary text-center mb-4">What would you like to do?</h3>
    <form class="w-full flex flex-col items-center gap-3" @submit.prevent="$emit('submit')">
      <div
        class="rounded-xl border border-border-card bg-bg-elevated px-4 py-3 shadow-fluent-8 w-full max-w-[800px]"
      >
        <Input
          ref="intentInputRef"
          id="pipeline-intent-input"
          :model-value="props.modelValue"
          type="text"
          class="flex-1 border-0 bg-transparent text-sm text-text-primary shadow-none outline-none ring-0 placeholder:text-text-tertiary focus-visible:border-transparent focus-visible:ring-0 h-auto px-0 py-0 rounded-none w-full"
          placeholder="What do you want to do?"
          autocomplete="off"
          :disabled="props.submitting"
          :aria-invalid="props.error.length > 0"
          @update:model-value="(val: string | number) => emit('update:modelValue', String(val))"
        />
      </div>
      <p v-if="props.error" class="m-0 text-[0.75rem] text-center text-danger" role="alert">
        {{ props.error }}
      </p>
      <p class="m-0 text-[0.75rem] text-center text-text-secondary">
        Press Enter to start designing your pipeline.
      </p>
      <p v-if="!props.hasIntentAgent" class="m-0 text-[0.75rem] text-center text-danger" role="alert">
        Configure an agent before creating a pipeline.
      </p>
    </form>
  </section>
</template>
