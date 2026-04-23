<script setup lang="ts">
import type { Component } from 'vue';

defineProps<{
  displayLabel: string;
  failureSummary?: string | null;
  statusAppearance: string;
  typeIcon: Component;
  typeLabel: string;
}>();
</script>

<template>
  <div class="flex items-center justify-between gap-2 px-3 pt-[10px] pb-1">
    <div class="flex items-center gap-1.5 min-w-0">
      <span
        class="status-dot relative inline-block w-1.5 h-1.5 rounded-full shrink-0"
        :class="{
          'bg-[var(--color-status-pending)]': statusAppearance === 'pending' || statusAppearance === 'cancelled',
          'bg-[var(--color-status-active)] status-dot--active': statusAppearance === 'active',
          'bg-[var(--color-status-stuck)]': statusAppearance === 'stuck',
          'bg-[var(--color-status-completed)]': statusAppearance === 'completed',
          'bg-[var(--color-status-abandoned)]': statusAppearance === 'abandoned',
        }"
      />
      <span class="overflow-hidden text-ellipsis whitespace-nowrap text-[var(--color-text-primary)] [font-family:var(--font-display)] text-[length:var(--text-sm)] [font-weight:var(--weight-title)]">{{ displayLabel }}</span>
      <span
        v-if="failureSummary"
        class="text-[var(--color-status-abandoned)] text-[0.8rem] leading-none shrink-0 cursor-help"
        :title="failureSummary"
        aria-label="Node failure reason available"
      >
        ⚠
      </span>
    </div>
    <span class="overflow-hidden text-ellipsis inline-flex items-center gap-1 shrink-0 text-[var(--node-accent)] text-[length:var(--text-xs)] leading-[1.1] whitespace-nowrap">
      <component :is="typeIcon" class="size-4" :stroke-width="1.5" />
      {{ typeLabel }}
    </span>
  </div>
</template>

<!-- audit-ok: keyframe-animation — @keyframes and ::after pseudo-element for status dot pulse; not expressible as Tailwind utilities. -->
<style scoped>
.status-dot--active::after {
  content: '';
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  border: 1px solid color-mix(in srgb, var(--color-status-active) 72%, transparent);
  animation: status-dot-pulse 1.8s ease-out infinite;
}

@keyframes status-dot-pulse {
  0% {
    opacity: 0.8;
    transform: scale(0.85);
  }
  70% {
    opacity: 0;
    transform: scale(1.8);
  }
  100% {
    opacity: 0;
    transform: scale(1.8);
  }
}
</style>
