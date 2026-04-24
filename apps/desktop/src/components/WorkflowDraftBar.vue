<script setup lang="ts">
import { Zap } from 'lucide-vue-next'
import { ref, watch } from 'vue'
import { Badge } from '@/components/ui/badge'
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
</script>

<template>
  <div
    class="min-h-9 flex items-center gap-3 px-4 bg-surface-highest shrink-0 border-t border-warning/30"
    role="status"
    aria-live="polite"
  >
    <template v-if="confirmingDiscard">
      <Zap class="size-3.5 text-warning shrink-0" :stroke-width="1.5" />
      <span class="flex-1 font-mono text-xs font-semibold text-warning tracking-wider">
        Unsaved changes — discard all?
      </span>
      <Button
        variant="ghost"
        size="sm"
        type="button"
        class="h-6 px-2 text-xs"
        @click="hasPendingSelection ? emit('cancel-pending') : (confirmingDiscard = false)"
      >
        Cancel
      </Button>
      <Button
        variant="destructive"
        size="sm"
        type="button"
        class="h-6 px-2 text-xs"
        @click="emit('discard')"
      >
        Yes, Discard
      </Button>
    </template>
    <template v-else>
      <Zap class="size-3.5 text-warning shrink-0" :stroke-width="1.5" />
      <span class="flex-1 font-mono text-xs font-semibold text-warning tracking-wider">
        {{ origin === 'packaged' ? 'Packaged workflow — will save as copy' : 'Unsaved changes' }}
      </span>
      <Badge
        v-if="saveState === 'error' && saveMessage"
        variant="destructive"
        class="text-xs font-normal"
      >
        {{ saveMessage }}
      </Badge>
      <Badge
        v-else-if="saveState === 'saving'"
        variant="outline"
        class="text-xs font-normal text-text-secondary"
      >
        Saving…
      </Badge>
      <Badge
        v-else-if="saveState === 'success'"
        variant="outline"
        class="text-xs font-normal border-success text-success"
      >
        Saved ✓
      </Badge>
      <Button
        variant="ghost"
        size="sm"
        type="button"
        class="h-6 px-2 text-xs"
        @click="confirmingDiscard = true"
      >
        Discard
      </Button>
      <Button
        variant="outline"
        size="sm"
        type="button"
        class="h-6 px-2 text-xs border-warning text-warning hover:bg-warning/10 hover:text-warning"
        :disabled="saveState === 'saving'"
        @click="emit('save')"
      >
        {{ saveState === 'error' ? 'Retry' : origin === 'packaged' ? 'Save Copy' : 'Save' }}
      </Button>
    </template>
  </div>
</template>
