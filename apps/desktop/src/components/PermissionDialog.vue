<script setup lang="ts">
// Lifted from formulahendry/acp-ui (MIT) — see LICENSE-acp-ui
// https://github.com/formulahendry/acp-ui
// Adaptation notes: Maenifold adaptation: replace scoped CSS with Tailwind design tokens and shadcn-vue Button.

import { Button } from '@/components/ui/button';
import type { PermissionRequest } from '../lib/types';

defineProps<{
  request: PermissionRequest;
}>();

const emit = defineEmits<{
  select: [optionId: string];
  cancel: [];
}>();

function handleSelect(optionId: string) {
  emit('select', optionId);
}

function handleCancel() {
  emit('cancel');
}

function optionVariant(kind: string): 'default' | 'destructive' | 'outline' {
  if (kind.startsWith('allow')) return 'default';
  if (kind.startsWith('reject')) return 'destructive';
  return 'outline';
}
</script>

<template>
  <div class="permission-overlay fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="permission-dialog w-[90%] max-w-md overflow-hidden rounded-lg border border-border-subtle bg-bg-surface shadow-xl">
      <div class="dialog-header flex items-center gap-2 border-b border-border-subtle bg-surface px-4 py-4">
        <span class="icon text-2xl">🔐</span>
        <h3 class="m-0 text-lg font-semibold text-text-primary">Permission Required</h3>
      </div>

      <div class="dialog-content p-4">
        <div class="tool-info flex flex-col gap-1">
          <span class="tool-title text-base font-semibold text-text-primary">{{ request.toolCall.title }}</span>
          <span class="tool-kind text-sm capitalize text-text-secondary">{{ request.toolCall.kind }}</span>
        </div>

        <div v-if="request.toolCall.locations?.length" class="locations mt-3 rounded border border-border-subtle bg-surface-high px-3 py-2">
          <div
            v-for="(loc, index) in request.toolCall.locations"
            :key="index"
            class="location break-all py-0.5 font-mono text-xs text-text-primary"
          >
            📁 {{ loc.path }}
          </div>
        </div>
      </div>

      <div class="dialog-actions flex flex-wrap gap-2 border-t border-border-subtle bg-surface px-4 py-4">
        <Button
          v-for="option in request.options"
          :key="option.optionId"
          :class="['option-btn min-w-28 flex-1', `option-${option.kind}`]"
          :variant="optionVariant(option.kind)"
          @click="handleSelect(option.optionId)"
        >
          {{ option.name }}
        </Button>
        <Button class="cancel-btn min-w-28 flex-1" variant="outline" @click="handleCancel">
          Cancel
        </Button>
      </div>
    </div>
  </div>
</template>
