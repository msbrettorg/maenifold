<script setup lang="ts">
defineProps<{
  displayLabel: string;
  nodeType: string;
  statusLabel: string;
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <header class="zoom-header">
    <div class="zoom-header-left">
      <span class="zoom-node-id">{{ displayLabel }}</span>
      <span :class="['zoom-type-pill', `zoom-type--${nodeType}`]">{{ nodeType }}</span>
      <span :class="['zoom-status', `zoom-status--${statusLabel}`]">{{ statusLabel }}</span>
    </div>
    <button class="zoom-close" aria-label="Close inspection" title="Close (Esc)" @click="$emit('close')">
      ×
    </button>
  </header>
</template>

<style scoped>
.zoom-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  background: var(--accent, #353534);
}

.zoom-header-left {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.zoom-node-id {
  font-family: var(--font-code);
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
  color: var(--foreground);
}

.zoom-type-pill,
.zoom-status {
  border-radius: 4px;
  font-size: var(--text-tiny);
}

.zoom-type-pill {
  padding: 2px 8px;
  color: var(--foreground);
  background: color-mix(in srgb, var(--color-node-workflow) 15%, transparent);
}

.zoom-type--prompt {
  background: color-mix(in srgb, var(--color-node-prompt) 15%, transparent);
}

.zoom-type--sequentialThinking {
  background: color-mix(in srgb, var(--color-node-thinking) 15%, transparent);
}

.zoom-type--tool {
  background: color-mix(in srgb, var(--color-node-tool) 15%, transparent);
}

.zoom-status {
  padding: 2px 6px;
  color: var(--muted-foreground);
  background: color-mix(in srgb, var(--color-status-pending) 15%, transparent);
}

.zoom-status--active { background: color-mix(in srgb, var(--color-status-active) 15%, transparent); color: var(--color-status-active); }
.zoom-status--completed { background: color-mix(in srgb, var(--color-status-completed) 15%, transparent); color: var(--color-status-completed); }
.zoom-status--abandoned { background: color-mix(in srgb, var(--color-status-abandoned) 15%, transparent); color: var(--color-status-abandoned); }
.zoom-status--ready { background: color-mix(in srgb, var(--color-status-ready) 15%, transparent); color: var(--color-status-ready); }

.zoom-close {
  background: none;
  border: none;
  color: var(--muted-foreground);
  font-size: var(--text-title);
  cursor: pointer;
  padding: var(--space-1);
  line-height: 1;
  border-radius: var(--radius-sm);
}

.zoom-close:hover {
  color: var(--foreground);
  background: var(--muted);
}

.zoom-close:focus-visible {
  outline: 2px solid var(--ring);
  outline-offset: 2px;
}
</style>
