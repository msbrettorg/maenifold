<script setup lang="ts">
defineProps<{
  isLoading: boolean;
  error: string;
}>();

defineEmits<{ retry: [] }>();
</script>

<template>
  <div class="graph-canvas-wrap" role="region" aria-label="Knowledge graph canvas">
    <div v-if="error && !isLoading" class="graph-empty-state" role="alert">
      <div class="empty-icon">◎</div>
      <p class="empty-title">No graph connection</p>
      <p class="empty-hint">Launch maenifold to explore the knowledge graph.</p>
      <button class="empty-retry" @click="$emit('retry')">Retry</button>
    </div>

    <div v-else-if="isLoading" class="graph-loading" role="status" aria-live="polite">
      <div class="loading-ring" />
      <p class="loading-text">Loading graph…</p>
    </div>

    <slot />
  </div>
</template>

<style scoped>
.graph-canvas-wrap {
  position: relative;
  flex: 1;
  overflow: hidden;
  background:
    radial-gradient(ellipse at 30% 50%, rgba(77, 142, 255, 0.03) 0%, transparent 60%),
    radial-gradient(ellipse at 70% 40%, rgba(78, 222, 163, 0.02) 0%, transparent 50%),
    var(--background);
}

:slotted(.graph-canvas) {
  position: relative;
  width: 100%;
  height: 100%;
  background: var(--background);
}

.graph-empty-state,
.graph-loading {
  position: absolute;
  inset: 0;
  z-index: 2;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.graph-empty-state {
  background: radial-gradient(ellipse at center, rgba(77, 142, 255, 0.03) 0%, transparent 70%);
}

.empty-icon {
  margin-bottom: 1rem;
  font-size: var(--text-display);
  color: var(--muted-foreground);
  opacity: 0.3;
}

.empty-title {
  margin-bottom: 0.25rem;
  font-family: var(--font-display);
  font-size: var(--text-lg);
  color: var(--muted-foreground);
}

.empty-hint,
.loading-text {
  font-size: var(--text-caption);
  color: var(--muted-foreground);
}

.empty-hint {
  margin-bottom: 1.25rem;
}

.empty-retry {
  padding: 0.4rem 1.2rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--muted);
  color: var(--muted-foreground);
  cursor: pointer;
  font-size: var(--text-caption);
  transition: var(--transition-fast);
}

.empty-retry:hover {
  background: var(--muted);
  color: var(--foreground);
  border-color: var(--ring);
}

.loading-ring {
  width: 32px;
  height: 32px;
  margin-bottom: 0.75rem;
  border: 2px solid var(--border);
  border-top-color: var(--ring);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
