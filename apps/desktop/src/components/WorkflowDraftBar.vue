<script setup lang="ts">
import { Zap } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
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

// Shared button base classes (re-applied via :class to keep template lines readable)
const btnBase =
  'min-h-6 px-2 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-transparent text-[var(--color-text-primary)] [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-body)] cursor-pointer disabled:cursor-wait disabled:opacity-70'

const ghostBtn = computed(() => `${btnBase} hover:bg-[var(--color-bg-surface)]`)

const labelMessage = computed(() =>
  'flex-1 [font-weight:var(--weight-title)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-code)] text-[var(--color-status-active)] tracking-[0.04em]',
)
</script>

<template>
  <!-- audit-ok: dynamic-binding — top border uses color-mix() with --color-status-active -->
  <div
    class="min-h-9 flex items-center gap-3 px-4 bg-[var(--color-bg-elevated)] shrink-0"
    :style="{ borderTop: '1px solid color-mix(in srgb, var(--color-status-active) 30%, var(--color-border-subtle))' }"
    role="status"
    aria-live="polite"
  >
    <template v-if="confirmingDiscard">
      <span class="text-[var(--color-status-active)] text-[length:var(--text-caption)]"><Zap class="inline size-3.5" :stroke-width="1.5" /></span>
      <span :class="labelMessage">Unsaved changes — discard all?</span>
      <Button variant="ghost" :class="ghostBtn" type="button" @click="hasPendingSelection ? emit('cancel-pending') : (confirmingDiscard = false)">
        Cancel
      </Button>
      <!-- audit-ok: dynamic-binding — danger hover uses color-mix() -->
      <Button
        variant="destructive"
        :class="`${btnBase} border-[var(--color-status-abandoned)] text-[var(--color-status-abandoned)]`"
        :style="{ '--tw-bg-hover': 'color-mix(in srgb, var(--color-status-abandoned) 14%, transparent)' }"
        type="button"
        @click="emit('discard')"
      >
        Yes, Discard
      </Button>
    </template>
    <template v-else>
      <span class="text-[var(--color-status-active)] text-[length:var(--text-caption)]"><Zap class="inline size-3.5" :stroke-width="1.5" /></span>
      <span :class="labelMessage">
        {{ origin === 'packaged' ? 'Packaged workflow — will save as copy' : 'Unsaved changes' }}
      </span>
      <span
        v-if="saveState === 'error' && saveMessage"
        class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-body)] text-[var(--color-status-abandoned)]"
      >
        {{ saveMessage }}
      </span>
      <span
        v-else-if="saveState === 'saving'"
        class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-body)] text-[var(--color-text-secondary)]"
      >Saving…</span>
      <span
        v-else-if="saveState === 'success'"
        class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-body)] text-[var(--color-status-active)]"
      >
        Saved ✓
      </span>
      <Button variant="ghost" :class="ghostBtn" type="button" @click="confirmingDiscard = true">
        Discard
      </Button>
      <!-- audit-ok: dynamic-binding — accent hover uses color-mix() (handled via :style on hover) -->
      <Button
        variant="outline"
        :class="`${btnBase} border-[var(--color-status-active)] text-[var(--color-status-active)] hover:bg-[color-mix(in_srgb,var(--color-status-active)_14%,transparent)]`"
        :disabled="saveState === 'saving'"
        type="button"
        @click="emit('save')"
      >
        {{ saveState === 'error' ? 'Retry' : origin === 'packaged' ? 'Save Copy' : 'Save' }}
      </Button>
    </template>
  </div>
</template>
