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
  <div class="graph-toolbar" role="toolbar" aria-label="Knowledge graph controls">
    <div class="graph-breadcrumb">
      <button
        v-if="viewLevel === 'concepts'"
        class="breadcrumb-link"
        @click="$emit('navigateBack')"
      >
        ← Communities
      </button>
      <span v-if="viewLevel === 'concepts'" class="breadcrumb-sep">/</span>
      <span class="breadcrumb-current">
        {{ viewLevel === 'communities' ? 'Knowledge Graph' : activeCommunityAnchor }}
      </span>
    </div>

    <div v-if="nodeCount > 0" class="graph-stats">
      <span class="stat">{{ nodeCount }} nodes</span>
      <span class="stat-sep">·</span>
      <span class="stat">{{ edgeCount }} edges</span>
    </div>

    <div class="toolbar-actions">
      <button class="graph-action-btn" title="Reset view" @click="$emit('reset')">↺</button>
      <button class="graph-action-btn" title="Reload data" @click="$emit('reload')">⟳</button>
    </div>
  </div>
</template>

<style scoped>
.graph-toolbar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--border);
  background: var(--card);
  flex-shrink: 0;
}

.graph-breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex: 1;
  font-size: var(--text-caption);
  color: var(--muted-foreground);
}

.breadcrumb-link {
  padding: 0;
  border: none;
  background: none;
  color: var(--ring);
  cursor: pointer;
  font-size: var(--text-caption);
}

.breadcrumb-link:hover {
  text-decoration: underline;
}

.breadcrumb-sep {
  color: var(--muted-foreground);
}

.breadcrumb-current {
  color: var(--foreground);
  font-weight: var(--weight-display);
  font-family: var(--font-display);
  letter-spacing: 0.02em;
}

.graph-stats {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-tiny);
  color: var(--muted-foreground);
  font-variant-numeric: tabular-nums;
}

.stat-sep {
  opacity: 0.4;
}

.toolbar-actions {
  display: flex;
  gap: 0.25rem;
}

.graph-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--muted);
  color: var(--muted-foreground);
  cursor: pointer;
  font-size: var(--text-body-sm);
  line-height: 1;
  transition: var(--transition-fast);
}

.graph-action-btn:hover {
  background: var(--muted);
  color: var(--foreground);
  border-color: var(--muted-foreground);
}
</style>
