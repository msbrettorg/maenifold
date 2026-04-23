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
              'bg-[var(--sage)]': statusTone === 'ready',
              'bg-[var(--amber)]': statusTone === 'offline',
              'bg-[var(--terracotta)]': statusTone === 'error',
              'bg-[var(--text-secondary)]': statusTone === 'pending' || statusTone === 'idle',
            }"
            aria-hidden="true"
          />
          <span class="status-label text-[length:var(--text-tiny)] text-[var(--color-text-primary)]">{{ statusLabel }}</span>
        </span>
        <span
          v-if="statusMessage"
          class="status-message text-[length:var(--text-caption)] text-[var(--color-text-secondary)]"
        >{{ statusMessage }}</span>
      </div>
      <Button
        variant="outline"
        size="sm"
        class="refresh-btn border-none rounded-[4px] bg-[var(--surface-container)] text-[var(--color-text-secondary)] cursor-pointer text-[length:var(--text-tiny)] font-normal px-[0.65rem] py-[0.35rem] whitespace-nowrap disabled:cursor-wait disabled:opacity-70 enabled:hover:bg-[var(--surface-high)]"
        :disabled="refreshDisabled"
        @click="$emit('refresh')"
      >
        {{ refreshing ? 'Refreshing…' : 'Refresh' }}
      </Button>
    </div>

    <p
      v-if="warningMessage"
      class="warning-note m-0 text-[length:var(--text-caption)] text-[var(--color-text-primary)] px-[0.65rem] py-[0.55rem] rounded-[4px] bg-[color-mix(in_srgb,var(--bg-warning)_20%,var(--surface-container))]"
      role="status"
      aria-live="polite"
    >{{ warningMessage }}</p>
    <p
      v-if="degradedMessage"
      class="degraded-note m-0 text-[length:var(--text-caption)] text-[var(--color-text-primary)]"
    >{{ degradedMessage }}</p>
    <p
      v-if="technicalDetail"
      class="technical-detail m-0 text-[length:var(--text-caption)] text-[var(--color-text-secondary)] break-words"
    >{{ technicalDetail }}</p>
  </div>
</template>
