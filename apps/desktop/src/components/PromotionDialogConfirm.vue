<script setup lang="ts">
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
  <section class="dialog-section">
    <div class="section-header">
      <div>
        <h4>Confirm promotion</h4>
        <p>Review the destination and the generated content before saving to memory.</p>
      </div>
      <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('back')">Back</button>
    </div>

    <div class="confirm-summary">
      <div class="summary-row">
        <strong>Action</strong>
        <span>{{ selectedAction }}</span>
      </div>
      <div v-if="selectedAction === 'create'" class="summary-row">
        <strong>Title</strong>
        <span>{{ title }}</span>
      </div>
      <div v-if="selectedAction === 'create' && folder" class="summary-row">
        <strong>Folder</strong>
        <span>{{ folder }}</span>
      </div>
      <div v-if="selectedAction === 'create' && tags.length" class="summary-row">
        <strong>Tags</strong>
        <span>{{ tags.join(', ') }}</span>
      </div>
      <div v-if="selectedResult || selectedMemory" class="summary-row">
        <strong>Target note</strong>
        <span>{{ selectedMemory?.uri ?? selectedResult?.uri }}</span>
      </div>
    </div>

    <p v-if="selectedAction === 'update' && updateDisabledMessage" class="info-message">{{ updateDisabledMessage }}</p>

    <div class="memory-preview">
      <span>Final content</span>
      <pre>{{ nextPreview }}</pre>
    </div>

    <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

    <div class="dialog-actions">
      <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('close')">Cancel</button>
      <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('retry')">Retry</button>
      <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('back')">Edit</button>
      <button
        type="button"
        class="btn btn-primary"
        :disabled="busy || (selectedAction === 'update' && !canSubmitUpdate)"
        @click="emit('submit')"
      >
        {{ submitLabel }}
      </button>
    </div>
  </section>
</template>
