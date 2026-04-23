<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  isLoading: boolean;
  error: string;
  nodeCount: number;
}>();

defineEmits<{ retry: [] }>();
</script>

<template>
  <div class="graph-canvas-wrap" role="region" aria-label="Knowledge graph canvas">
    <div v-if="error && !isLoading" class="graph-empty-state" role="alert">
      <div class="empty-icon">◎</div>
      <p class="empty-title">No graph connection</p>
      <p class="empty-hint">Launch maenifold to explore the knowledge graph.</p>
      <Button variant="outline" size="sm" class="empty-retry" @click="$emit('retry')">Retry</Button>
    </div>

    <div v-else-if="isLoading" class="graph-loading" role="status" aria-live="polite">
      <div class="loading-ring" />
      <p class="loading-text">Building knowledge map…</p>
      <p v-if="nodeCount > 0" class="loading-detail">{{ nodeCount }} nodes discovered</p>
    </div>

    <slot />

    <div v-if="!isLoading && !error" class="graph-hints">
      Drag to pan · Scroll to zoom · Click a node for details
    </div>
  </div>
</template>

<style scoped>
.graph-canvas-wrap {
  position: relative;
  flex: 1;
  overflow: hidden;
  background:
    radial-gradient(ellipse at center, color-mix(in srgb, var(--color-text-primary) 3%, transparent) 0%, transparent 55%),
    radial-gradient(ellipse at center, var(--color-bg-app) 0%, color-mix(in srgb, var(--color-bg-app) 30%, #000) 40%, #000 100%);
}

:slotted(.graph-canvas) {
  position: relative;
  width: 100%;
  height: 100%;
  background: transparent;
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
  background: radial-gradient(ellipse at center, color-mix(in srgb, var(--color-brand) 3%, transparent) 0%, transparent 70%);
}

.empty-icon {
  margin-bottom: 1rem;
  font-size: var(--text-display);
  color: var(--color-text-secondary);
  opacity: 0.3;
}

.empty-title {
  margin-bottom: 0.25rem;
  font-family: var(--font-display);
  font-size: var(--text-lg);
  color: var(--color-text-secondary);
}

.empty-hint,
.loading-text {
  font-size: var(--text-caption);
  color: var(--color-text-secondary);
}

.empty-hint {
  margin-bottom: 1.25rem;
}

.empty-retry {
  padding: 0.4rem 1.2rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: 0.25rem;
  background: var(--surface-high);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: var(--text-caption);
  transition: var(--transition-fast);
}

.empty-retry:hover {
  background: var(--surface-highest);
  color: var(--color-text-primary);
}

.loading-ring {
  width: 32px;
  height: 32px;
  margin-bottom: 0.75rem;
  border: 2px solid var(--color-border-subtle);
  border-top-color: var(--color-brand);
  border-radius: var(--radius);
  animation: spin 0.8s linear infinite;
}

.loading-detail {
  margin-top: 0.25rem;
  font-size: 0.6875rem;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.graph-hints {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 1;
  padding: 0.5rem 1rem;
  text-align: center;
  font-size: 0.6875rem;
  color: var(--text-secondary);
  pointer-events: none;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
