<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  compact: boolean;
  degradedMessage: string | null;
  ready: boolean;
  refreshDisabled: boolean;
  refreshing: boolean;
  statusLabel: string;
  statusMessage: string;
  statusTone: string;
  technicalDetail: string | null;
  warningMessage: string | null | undefined;
}>();

defineEmits<{
  refresh: [];
}>();
</script>

<template>
  <div class="preview-status grid gap-[0.65rem]" :class="[statusTone, { compact }]">
    <div class="preview-header flex flex-wrap gap-[0.65rem] items-center justify-between">
      <div class="status-copy flex flex-wrap gap-[0.45rem] items-center">
        <span class="status-indicator inline-flex items-center gap-[0.5rem]">
          <span
            class="status-bead inline-block w-[6px] h-[6px] rounded-[4px]"
            :class="{
              'bg-success': statusTone === 'ready',
              'bg-warning': statusTone === 'offline',
              'bg-danger': statusTone === 'error',
              'bg-text-secondary': statusTone === 'pending' || statusTone === 'idle',
            }"
            aria-hidden="true"
          />
          <span class="status-label text-text-primary text-tiny">{{ statusLabel }}</span>
        </span>
        <span
          v-if="statusMessage"
          class="status-message text-caption text-text-secondary"
        >{{ statusMessage }}</span>
      </div>
      <Button
        variant="outline"
        size="sm"
        class="refresh-btn cursor-pointer rounded-[4px] border-none bg-surface-container px-[0.65rem] py-[0.35rem] text-text-secondary text-tiny font-normal whitespace-nowrap disabled:cursor-wait disabled:opacity-70 enabled:hover:bg-surface-high"
        :disabled="refreshDisabled"
        @click="$emit('refresh')"
      >
        {{ refreshing ? 'Refreshing…' : 'Refresh' }}
      </Button>
    </div>

    <p
      v-if="warningMessage"
      class="warning-note m-0 rounded-[4px] bg-bg-warning/20 px-[0.65rem] py-[0.55rem] text-caption text-text-primary"
      role="status"
      aria-live="polite"
    >{{ warningMessage }}</p>
    <p
      v-if="degradedMessage"
      class="degraded-note m-0 text-caption text-text-primary"
    >{{ degradedMessage }}</p>
    <p
      v-if="technicalDetail"
      class="technical-detail m-0 break-words text-caption text-text-secondary"
    >{{ technicalDetail }}</p>
  </div>
</template>
