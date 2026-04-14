<script setup lang="ts">
import { ref, watch } from 'vue'
import type { WorkflowOrigin } from '../stores/workflow-library-helpers'
import type { WorkflowSaveState } from '../stores/workflow-library-save-actions'

const props = defineProps<{
  hasPendingSelection: boolean
  isDirty: boolean
  origin: WorkflowOrigin | null
  saveMessage: string | null
  saveState: WorkflowSaveState
}>()

const emit = defineEmits<{
  discard: []
  'cancel-pending': []
  save: []
}>()

const confirmingDiscard = ref(false)

watch(
  () => props.hasPendingSelection,
  (hasPendingSelection) => {
    confirmingDiscard.value = hasPendingSelection
  },
  { immediate: true },
)

watch(
  () => props.isDirty,
  (isDirty) => {
    if (!isDirty && props.saveState !== 'error') {
      confirmingDiscard.value = false
    }
  },
)
</script>

<template>
  <div class="draft-bar" role="status" aria-live="polite">
    <template v-if="confirmingDiscard">
      <span class="draft-bar-icon">⚡</span>
      <span class="draft-bar-label">Unsaved changes — discard all?</span>
      <button class="draft-bar-btn draft-bar-btn--ghost" type="button" @click="hasPendingSelection ? emit('cancel-pending') : (confirmingDiscard = false)">
        Cancel
      </button>
      <button class="draft-bar-btn draft-bar-btn--danger" type="button" @click="emit('discard')">
        Yes, Discard
      </button>
    </template>
    <template v-else>
      <span class="draft-bar-icon">⚡</span>
      <span class="draft-bar-label">
        {{ origin === 'packaged' ? 'Packaged workflow — will save as copy' : 'Unsaved changes' }}
      </span>
      <span v-if="saveState === 'error' && saveMessage" class="draft-bar-message draft-bar-message--error">
        {{ saveMessage }}
      </span>
      <span v-else-if="saveState === 'saving'" class="draft-bar-message">Saving…</span>
      <span v-else-if="saveState === 'success'" class="draft-bar-message draft-bar-message--success">
        Saved ✓
      </span>
      <button class="draft-bar-btn draft-bar-btn--ghost" type="button" @click="confirmingDiscard = true">
        Discard
      </button>
      <button
        class="draft-bar-btn draft-bar-btn--accent"
        :disabled="saveState === 'saving'"
        type="button"
        @click="emit('save')"
      >
        {{ saveState === 'error' ? 'Retry' : origin === 'packaged' ? 'Save Copy' : 'Save' }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.draft-bar {
  min-height: 36px;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-4);
  background: var(--card);
  border-top: 1px solid color-mix(in srgb, var(--color-status-active) 30%, var(--border));
  flex-shrink: 0;
}

.draft-bar-icon {
  color: var(--color-status-active);
  font-size: var(--text-caption);
}

.draft-bar-label {
  flex: 1;
  font: var(--weight-title) var(--text-tiny)/1 var(--font-code);
  color: var(--color-status-active);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.draft-bar-message {
  font: var(--weight-display) var(--text-tiny)/1.4 var(--font-body);
  color: var(--muted-foreground);
}

.draft-bar-message--error {
  color: var(--color-status-abandoned);
}

.draft-bar-message--success {
  color: var(--color-status-active);
}

.draft-bar-btn {
  min-height: 24px;
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: transparent;
  color: var(--foreground);
  font: var(--weight-display) var(--text-tiny)/1 var(--font-body);
  cursor: pointer;
}

.draft-bar-btn:disabled {
  cursor: wait;
  opacity: 0.7;
}

.draft-bar-btn--ghost:hover {
  background: var(--muted);
}

.draft-bar-btn--accent {
  border-color: var(--color-status-active);
  color: var(--color-status-active);
}

.draft-bar-btn--accent:hover {
  background: color-mix(in srgb, var(--color-status-active) 14%, transparent);
}

.draft-bar-btn--danger {
  border-color: var(--color-status-abandoned);
  color: var(--color-status-abandoned);
}

.draft-bar-btn--danger:hover {
  background: color-mix(in srgb, var(--color-status-abandoned) 14%, transparent);
}
</style>
