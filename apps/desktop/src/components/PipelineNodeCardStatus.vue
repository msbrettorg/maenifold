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
  <div class="node-header">
    <div class="header-main">
      <span :class="['status-dot', `status-dot--${statusAppearance}`]" />
      <span class="node-title">{{ displayLabel }}</span>
      <span
        v-if="failureSummary"
        class="failure-icon"
        :title="failureSummary"
        aria-label="Node failure reason available"
      >
        ⚠
      </span>
    </div>
    <span class="type-pill">
      <component :is="typeIcon" class="size-4" :stroke-width="1.5" />
      {{ typeLabel }}
    </span>
  </div>
</template>

<style scoped>
.node-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
   padding: 10px 12px 4px;
}

.header-main {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.status-dot {
  position: relative;
  width: 6px;
  height: 6px;
  border-radius: 999px;
  flex-shrink: 0;
  display: inline-block;
}

.status-dot--pending,
.status-dot--cancelled { background: var(--color-status-pending); }
.status-dot--active,
.status-dot--stuck { background: var(--color-status-active); }
.status-dot--completed { background: var(--color-status-completed); }
.status-dot--abandoned { background: var(--color-status-abandoned); }

.status-dot--active::after {
  content: '';
  position: absolute;
  inset: -3px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--color-status-active) 72%, transparent);
  animation: status-dot-pulse 1.8s ease-out infinite;
}

.node-title,
.type-pill {
  overflow: hidden;
  text-overflow: ellipsis;
}

.node-title {
  color: var(--foreground);
  white-space: nowrap;
   font-family: var(--font-display);
   font-size: var(--text-sm);
   font-weight: var(--weight-title);
}

.type-pill {
   display: inline-flex;
   align-items: center;
   gap: 0.25rem;
   flex-shrink: 0;
   color: var(--node-accent);
   font-size: var(--text-xs);
   line-height: 1.1;
   white-space: nowrap;
}

.failure-icon {
  color: var(--color-status-abandoned);
  font-size: 0.8rem;
  line-height: 1;
  flex-shrink: 0;
  cursor: help;
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
