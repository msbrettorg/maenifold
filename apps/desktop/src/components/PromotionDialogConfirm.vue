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
        <h4 class="m-0 font-display text-base font-medium text-text-primary">Confirm promotion</h4>
        <p class="text-text-secondary">Review the destination and the generated content before saving to memory.</p>
      </div>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('back')">Back</Button>
    </div>

    <div class="grid gap-4">
      <div class="flex items-start justify-between gap-3 rounded-[4px] bg-surface p-3.5 max-[720px]:flex-col">
        <strong class="text-xs text-text-primary">Action</strong>
        <span class="text-xs text-text-primary">{{ selectedAction }}</span>
      </div>
      <div v-if="selectedAction === 'create'" class="flex items-start justify-between gap-3 rounded-[4px] bg-surface p-3.5 max-[720px]:flex-col">
        <strong class="text-xs text-text-primary">Title</strong>
        <span class="text-xs text-text-primary">{{ title }}</span>
      </div>
      <div v-if="selectedAction === 'create' && folder" class="flex items-start justify-between gap-3 rounded-[4px] bg-surface p-3.5 max-[720px]:flex-col">
        <strong class="text-xs text-text-primary">Folder</strong>
        <span class="text-xs text-text-primary">{{ folder }}</span>
      </div>
      <div v-if="selectedAction === 'create' && tags.length" class="flex items-start justify-between gap-3 rounded-[4px] bg-surface p-3.5 max-[720px]:flex-col">
        <strong class="text-xs text-text-primary">Tags</strong>
        <span class="text-xs text-text-primary">{{ tags.join(', ') }}</span>
      </div>
      <div v-if="selectedResult || selectedMemory" class="flex items-start justify-between gap-3 rounded-[4px] bg-surface p-3.5 max-[720px]:flex-col">
        <strong class="text-xs text-text-primary">Target note</strong>
        <span class="text-xs text-text-primary">{{ selectedMemory?.uri ?? selectedResult?.uri }}</span>
      </div>
    </div>

    <p v-if="selectedAction === 'update' && updateDisabledMessage" class="text-text-secondary">{{ updateDisabledMessage }}</p>

    <div class="grid gap-4">
      <span class="text-xs text-text-primary">Final content</span>
      <pre class="m-0 rounded-[4px] bg-surface p-3 font-[inherit] whitespace-pre-wrap break-words">{{ nextPreview }}</pre>
    </div>

    <p v-if="formError" class="m-0 text-status-abandoned text-xs" role="alert">{{ formError }}</p>

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
