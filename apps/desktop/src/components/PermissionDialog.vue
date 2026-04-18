<script setup lang="ts">
import { computed } from 'vue';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import type { PermissionRequest } from '../lib/types';
import PermissionEntry from './PermissionEntry.vue';

const props = defineProps<{ request: PermissionRequest; totalPending?: number }>();
const emit = defineEmits<{ select: [optionId: string]; cancel: [] }>();

const pendingCount = computed(() => props.totalPending ?? 0);
const hasQueue = computed(() => pendingCount.value > 1);
const toolName = computed(() => props.request.toolCall.title || props.request.toolCall.kind || 'Tool call');
const titleText = computed(() =>
  hasQueue.value ? `Permission 1 of ${pendingCount.value} — ${toolName.value}` : 'Permission required'
);
const pendingChip = computed(() => (hasQueue.value ? `${pendingCount.value} pending` : null));

const queueHint = computed(() =>
  hasQueue.value
    ? `The remaining requests will appear after you respond.`
    : null
);
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && emit('cancel')">
    <DialogContent class="permission-dialog gap-0 p-0 sm:max-w-3xl" :show-close-button="false">
      <DialogHeader class="dialog-header">
        <div class="dialog-title-row">
          <span class="icon" aria-hidden="true">🔐</span>
          <DialogTitle>{{ titleText }}</DialogTitle>
          <span v-if="pendingChip" class="pending-chip">{{ pendingChip }}</span>
        </div>
        <DialogDescription class="queue-hint">
          Review the pending permission request and choose how to respond.
          <template v-if="queueHint"> {{ queueHint }}</template>
        </DialogDescription>
      </DialogHeader>

      <div class="dialog-content">
        <div class="permission-list" role="list" aria-label="Pending permission requests">
          <PermissionEntry
            :request="request"
            @select="emit('select', $event)"
            @cancel="emit('cancel')"
          />
        </div>
      </div>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.permission-dialog {
  display: flex;
  flex-direction: column;
  width: min(48rem, 100%);
  max-height: min(42rem, calc(100vh - 2rem));
  overflow: hidden;
}

.dialog-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.dialog-header {
  display: grid;
  gap: 0.5rem;
  align-items: flex-start;
  padding: 1rem;
  background: var(--muted);
}

.dialog-header .icon {
  font-size: var(--text-title);
}

.dialog-content {
  flex: 1;
  min-height: 0;
  padding: 1rem;
  overflow: hidden;
}

.queue-hint {
  margin: 0;
  font-size: var(--text-caption);
  color: var(--muted-foreground);
}

.pending-chip {
  margin-left: auto;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  background: #2a2a2a;
  color: #c2c6d6;
  font-size: var(--text-tiny);
}

.permission-list {
  min-height: 0;
}
</style>
