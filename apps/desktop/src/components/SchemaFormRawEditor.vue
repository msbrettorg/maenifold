<script setup lang="ts">
import { Textarea } from '@/components/ui/textarea';

const props = defineProps<{
  ariaLabel?: string;
  'aria-label'?: string;
  error: string;
  message: string;
  title: string;
  value: string;
}>();

const emit = defineEmits<{
  input: [value: string];
}>();

function handleInput(event: Event): void {
  emit('input', (event.target as HTMLTextAreaElement).value);
}
</script>

<template>
  <div
    class="flex flex-col gap-4 p-4 border border-[var(--color-border-subtle)] rounded-lg bg-[var(--color-bg-elevated)]"
    role="group"
    :aria-label="props.ariaLabel ?? props['aria-label']"
  >
    <div class="flex flex-col gap-1">
      <h3 class="text-sm font-semibold">{{ props.title }}</h3>
      <p class="text-xs text-[var(--color-text-secondary)]">{{ props.message }}</p>
    </div>
    <Textarea
      class="min-h-64 font-mono"
      :value="props.value"
      rows="12"
      spellcheck="false"
      @input="handleInput"
    />
    <p v-if="props.error" class="text-xs text-[var(--color-danger)]">{{ props.error }}</p>
  </div>
</template>
