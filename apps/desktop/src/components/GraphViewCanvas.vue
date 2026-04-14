<script setup lang="ts">
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
      <button class="empty-retry" @click="$emit('retry')">Retry</button>
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
    radial-gradient(ellipse at 30% 50%, rgba(77, 142, 255, 0.03) 0%, transparent 60%),
    radial-gradient(ellipse at 70% 40%, rgba(78, 222, 163, 0.02) 0%, transparent 50%),
    #0e0e0e;
}

:slotted(.graph-canvas) {
  position: relative;
  width: 100%;
  height: 100%;
  background: #0e0e0e;
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
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: #2a2a2a;
  color: var(--muted-foreground);
  cursor: pointer;
  font-size: var(--text-caption);
  transition: var(--transition-fast);
}

.empty-retry:hover {
  background: #353534;
  color: #e5e2e1;
}

.loading-ring {
  width: 32px;
  height: 32px;
  margin-bottom: 0.75rem;
  border: 2px solid rgba(66, 71, 83, 0.15);
  border-top-color: #4d8eff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.loading-detail {
  margin-top: 0.25rem;
  font-size: 0.6875rem;
  color: #707070;
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
  color: #707070;
  pointer-events: none;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
