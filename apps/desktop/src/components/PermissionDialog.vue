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
    <DialogContent class="flex flex-col gap-0 p-0 sm:max-w-3xl w-[min(48rem,100%)] max-h-[min(42rem,calc(100vh-2rem))] overflow-hidden" :show-close-button="false">
      <DialogHeader class="grid gap-2 items-start p-4 bg-[var(--color-bg-surface)]">
        <div class="flex items-center gap-2">
          <span class="text-[length:var(--text-title)]" aria-hidden="true">🔐</span>
          <DialogTitle>{{ titleText }}</DialogTitle>
          <span v-if="pendingChip" class="ml-auto py-0.5 px-2 rounded-[var(--radius-xl)] bg-[var(--surface-high)] text-[var(--text-secondary)] text-[length:var(--text-tiny)]">{{ pendingChip }}</span>
        </div>
        <DialogDescription class="m-0 text-[length:var(--text-caption)] text-[var(--color-text-secondary)]">
          Review the pending permission request and choose how to respond.
          <template v-if="queueHint"> {{ queueHint }}</template>
        </DialogDescription>
      </DialogHeader>

      <div class="flex-1 min-h-0 p-4 overflow-hidden">
        <div class="min-h-0" role="list" aria-label="Pending permission requests">
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
