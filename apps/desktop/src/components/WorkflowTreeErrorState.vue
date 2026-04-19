<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next';
defineProps<{
  error: string;
}>();

defineEmits<{
  retry: [];
}>();
</script>

<template>
  <div class="tree-error" role="alert">
    <div class="error-icon"><AlertTriangle class="size-8 text-text-tertiary" :stroke-width="1.5" /></div>
    <div class="error-body">
      <p class="error-title">Failed to load workflow tree</p>
      <p class="error-detail">{{ error }}</p>
    </div>
    <button type="button" class="retry-btn" @click="$emit('retry')">
      ↺ Retry
    </button>
  </div>
</template>

<style scoped>
.tree-error {
  --tree-error-text: color-mix(in srgb, var(--color-text-primary) 74%, var(--color-danger));
  --tree-error-detail: color-mix(in srgb, var(--color-text-primary) 62%, var(--color-danger));
  display: flex;
  align-items: flex-start;
  gap: 0.875rem;
  max-width: 480px;
  background: color-mix(in srgb, var(--color-status-abandoned) 12%, var(--color-bg-surface));
  border: 1px solid color-mix(in srgb, var(--color-status-abandoned) 25%, transparent);
  border-radius: var(--radius);
  padding: 1rem 1.25rem;
  color: var(--tree-error-text);
}

.error-icon {
  flex-shrink: 0;
  font-size: var(--text-title);
  line-height: 1.3;
}

.error-body {
  flex: 1;
  min-width: 0;
}

.error-title {
  margin: 0 0 0.25rem;
  color: var(--tree-error-text);
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
}

.error-detail {
  margin: 0;
  color: var(--tree-error-detail);
  font-size: var(--text-caption);
  font-family: var(--font-code);
  overflow-wrap: anywhere;
}

.retry-btn {
  flex-shrink: 0;
  align-self: center;
  background: color-mix(in srgb, var(--color-status-abandoned) 18%, var(--color-bg-surface));
  border: 1px solid color-mix(in srgb, var(--color-status-abandoned) 30%, transparent);
  border-radius: var(--radius);
  padding: 0.375rem 0.75rem;
  color: var(--tree-error-text);
  cursor: pointer;
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
  white-space: nowrap;
  transition: background 0.15s ease;
}

.retry-btn:hover {
  background: color-mix(in srgb, var(--color-status-abandoned) 26%, var(--color-bg-surface));
}
</style>
