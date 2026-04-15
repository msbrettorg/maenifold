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
   padding: 10px 12px 4px;
}

.header-main {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.status-dot {
   width: 6px;
   height: 6px;
   border-radius: 4px;
   flex-shrink: 0;
}

.status-dot--pending,
.status-dot--cancelled { background: #707070; }
.status-dot--active,
.status-dot--stuck { background: #f59e0b; }
.status-dot--completed { background: #4edea3; }
.status-dot--abandoned { background: #ffb4ab; }

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
</style>
