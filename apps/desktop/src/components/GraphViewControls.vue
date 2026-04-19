<script setup lang="ts">
defineProps<{
  viewLevel: 'communities' | 'concepts';
  activeCommunityAnchor: string | null;
  nodeCount: number;
  edgeCount: number;
}>();

defineEmits<{
  navigateBack: [];
  reset: [];
  reload: [];
}>();
</script>

<template>
  <div class="graph-header" role="toolbar" aria-label="Knowledge graph controls">
    <div class="header-left">
      <button
        v-if="viewLevel === 'concepts'"
        class="back-btn"
        title="Back to communities"
        @click="$emit('navigateBack')"
      >
        ←
      </button>
      <span v-if="viewLevel === 'concepts' && activeCommunityAnchor" class="header-context">
        {{ activeCommunityAnchor }}
      </span>
      <span v-if="nodeCount > 0" class="header-stats">
        {{ nodeCount }} nodes · {{ edgeCount }} edges
      </span>
    </div>

    <div class="header-actions">
      <button class="action-btn" title="Reset viewport" @click="$emit('reset')">↺ Reset</button>
      <button class="action-btn" title="Reload graph data" @click="$emit('reload')">⟳ Reload</button>
    </div>
  </div>
</template>

<style scoped>
.graph-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  background: var(--surface);
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
}

.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: var(--surface-high);
  color: var(--accent-prompt);
  cursor: pointer;
  font-size: var(--text-caption);
  line-height: 1;
  transition: var(--transition-fast);
}

.back-btn:hover {
  background: var(--surface-highest);
  color: var(--color-text-primary);
}

.header-stats {
  font-size: 0.6875rem;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.header-context {
  font-size: 0.6875rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.header-actions {
  display: flex;
  gap: 0.375rem;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: var(--surface-high);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.6875rem;
  line-height: 1;
  white-space: nowrap;
  transition: var(--transition-fast);
}

.action-btn:hover {
  background: var(--surface-highest);
  color: var(--color-text-primary);
}
</style>
