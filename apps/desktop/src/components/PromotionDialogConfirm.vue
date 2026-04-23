<script setup lang="ts">
import { Button } from '@/components/ui/button';
import type {
  NormalizedMemoryDocument,
  NormalizedMemorySearchItem,
} from '../composables/useMemoryPromotion';

type PromotionAction = 'create' | 'append' | 'update';

defineProps<{
  selectedAction: PromotionAction | null;
  selectedResult: NormalizedMemorySearchItem | null;
  selectedMemory: NormalizedMemoryDocument | null;
  title: string;
  folder: string;
  tags: string[];
  nextPreview: string;
  formError: string;
  submitLabel: string;
  busy: boolean;
  canSubmitUpdate: boolean;
  updateDisabledMessage: string;
}>();

const emit = defineEmits<{
  (event: 'back'): void;
  (event: 'submit'): void;
  (event: 'retry'): void;
  (event: 'close'): void;
}>();
</script>

<template>
  <section class="grid gap-4 px-4 pb-4">
    <div class="flex gap-3 max-[720px]:flex-col">
      <div>
        <h4 class="m-0 font-display font-medium text-base text-[var(--color-text-primary)]">Confirm promotion</h4>
        <p class="text-[var(--text-secondary)]">Review the destination and the generated content before saving to memory.</p>
      </div>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('back')">Back</Button>
    </div>

    <div class="grid gap-4">
      <div class="flex gap-3 justify-between items-start rounded-[4px] p-3.5 bg-[var(--surface)] max-[720px]:flex-col">
        <strong class="text-[var(--color-text-primary)] text-xs">Action</strong>
        <span class="text-[var(--color-text-primary)] text-xs">{{ selectedAction }}</span>
      </div>
      <div v-if="selectedAction === 'create'" class="flex gap-3 justify-between items-start rounded-[4px] p-3.5 bg-[var(--surface)] max-[720px]:flex-col">
        <strong class="text-[var(--color-text-primary)] text-xs">Title</strong>
        <span class="text-[var(--color-text-primary)] text-xs">{{ title }}</span>
      </div>
      <div v-if="selectedAction === 'create' && folder" class="flex gap-3 justify-between items-start rounded-[4px] p-3.5 bg-[var(--surface)] max-[720px]:flex-col">
        <strong class="text-[var(--color-text-primary)] text-xs">Folder</strong>
        <span class="text-[var(--color-text-primary)] text-xs">{{ folder }}</span>
      </div>
      <div v-if="selectedAction === 'create' && tags.length" class="flex gap-3 justify-between items-start rounded-[4px] p-3.5 bg-[var(--surface)] max-[720px]:flex-col">
        <strong class="text-[var(--color-text-primary)] text-xs">Tags</strong>
        <span class="text-[var(--color-text-primary)] text-xs">{{ tags.join(', ') }}</span>
      </div>
      <div v-if="selectedResult || selectedMemory" class="flex gap-3 justify-between items-start rounded-[4px] p-3.5 bg-[var(--surface)] max-[720px]:flex-col">
        <strong class="text-[var(--color-text-primary)] text-xs">Target note</strong>
        <span class="text-[var(--color-text-primary)] text-xs">{{ selectedMemory?.uri ?? selectedResult?.uri }}</span>
      </div>
    </div>

    <p v-if="selectedAction === 'update' && updateDisabledMessage" class="text-[var(--text-secondary)]">{{ updateDisabledMessage }}</p>

    <div class="grid gap-4">
      <span class="text-[var(--color-text-primary)] text-xs">Final content</span>
      <pre class="m-0 p-3 rounded-[4px] bg-[var(--surface)] whitespace-pre-wrap break-words font-[inherit]">{{ nextPreview }}</pre>
    </div>

    <p v-if="formError" class="m-0 text-[var(--color-status-abandoned)] text-xs" role="alert">{{ formError }}</p>

    <div class="flex flex-wrap items-center gap-3 max-[720px]:justify-stretch [&>:first-child:not(:only-child)]:mr-auto">
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('close')">Cancel</Button>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('retry')">Retry</Button>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('back')">Edit</Button>
      <Button
        type="button"
        :disabled="busy || (selectedAction === 'update' && !canSubmitUpdate)"
        @click="emit('submit')"
      >
        {{ submitLabel }}
      </Button>
    </div>
  </section>
</template>
