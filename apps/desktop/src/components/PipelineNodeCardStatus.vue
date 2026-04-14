<script setup lang="ts">
defineProps<{
  displayLabel: string;
  statusAppearance: string;
  typeIcon: string;
  typeLabel: string;
}>();
</script>

<template>
  <div class="node-header">
    <div class="header-main">
      <span :class="['status-dot', `status-dot--${statusAppearance}`]" />
      <span class="node-title">{{ displayLabel }}</span>
    </div>
    <span class="type-pill">{{ typeIcon }} {{ typeLabel }}</span>
  </div>
</template>

<style scoped>
.node-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px 6px;
  background: color-mix(in srgb, var(--node-accent) 6%, transparent);
}

.header-main {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  flex-shrink: 0;
}

.node-title,
.type-pill {
  overflow: hidden;
  text-overflow: ellipsis;
}

.node-title {
  color: var(--foreground);
  white-space: nowrap;
  font-family: var(--font-code);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
}

.type-pill {
  display: inline-flex;
  align-items: center;
  min-height: 18px;
  padding: 2px 6px;
  border-radius: 999px;
  flex-shrink: 0;
  color: var(--node-accent);
  background: color-mix(in srgb, var(--node-accent) 10%, transparent);
  font-size: var(--text-xs);
  line-height: 1.1;
  white-space: nowrap;
}
</style>
