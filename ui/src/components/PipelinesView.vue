<script setup lang="ts">
// Pipelines tab — foundation placeholder shell (T-600.1).
//
// The real DAG canvas, browser flyout, inspector and surgery surfaces land in
// the follow-on T-600.2+ tasks. This shell only proves the tab is registered
// and the `pipelines` store RPC surface is reachable: on mount it performs a
// single read of `pipeline://sessions` and lists the pipelines it finds. No
// graph rendering, no polling, no behavioral state — those are deferred to the
// later tasks that extend this component and the store.
import { onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { GitBranch, RefreshCw } from '@lucide/vue';
import { usePipelinesStore } from '../stores/pipelines';

const store = usePipelinesStore();
const { pipelines, loading, error, pipelineCount } = storeToRefs(store);

onMounted(() => {
  void store.bootstrap();
});

function refresh(): void {
  void store.refreshList();
}
</script>

<template>
  <section class="pipelines-view">
    <div class="pipelines-pane">
      <header class="pane-header">
        <span class="pane-title">
          <GitBranch :size="15" :stroke-width="2" />
          <strong>Pipelines</strong>
        </span>
        <button
          type="button"
          class="pane-action"
          :disabled="loading"
          aria-label="Refresh pipelines"
          @click="refresh"
        >
          <RefreshCw :size="14" :stroke-width="2" :class="{ spinning: loading }" />
        </button>
      </header>

      <p v-if="error" class="pane-error">{{ error }}</p>

      <div class="pane-body">
        <p v-if="loading && pipelineCount === 0" class="pane-hint">Loading pipelines…</p>
        <p v-else-if="pipelineCount === 0" class="pane-hint">No pipelines yet.</p>
        <ul v-else class="pipeline-list">
          <li v-for="item in pipelines" :key="item.pipelineId" class="pipeline-row">
            <span class="pipeline-name">{{ item.name ?? item.pipelineId }}</span>
            <span class="pipeline-status">{{ item.status }}</span>
            <span class="pipeline-progress">
              {{ item.completedCount }}/{{ item.nodeCount }}
            </span>
          </li>
        </ul>
        <p class="pane-footnote">
          Canvas, inspector and surgery surfaces arrive in the next tasks.
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.pipelines-view {
  flex: 1;
  min-height: 0;
  display: flex;
  padding: 0.75rem;
  overflow: hidden;
}

.pipelines-pane {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  overflow: hidden;
}

.pane-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.65rem 0.75rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 68%, transparent);
  color: var(--text-secondary);
}

.pane-title {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.76rem;
  font-weight: 700;
  text-transform: uppercase;
}

.pane-title strong {
  color: var(--text-primary);
  font-size: 0.85rem;
  text-transform: none;
}

.pane-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.3rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.pane-action:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.pane-action:disabled {
  cursor: default;
  opacity: 0.6;
}

.spinning {
  animation: pipelines-spin 0.9s linear infinite;
}

@keyframes pipelines-spin {
  to {
    transform: rotate(360deg);
  }
}

.pane-error {
  margin: 0;
  padding: 0.55rem 0.75rem;
  border-bottom: 1px solid color-mix(in srgb, var(--bg-danger) 42%, var(--border-color));
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.8rem;
}

.pane-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem;
  overflow: auto;
}

.pane-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.85rem;
}

.pipeline-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.pipeline-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 0.6rem;
  padding: 0.5rem 0.65rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
}

.pipeline-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
  font-size: 0.85rem;
  font-weight: 500;
}

.pipeline-status {
  color: var(--text-accent);
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.pipeline-progress {
  color: var(--text-muted);
  font-size: 0.78rem;
  font-variant-numeric: tabular-nums;
}

.pane-footnote {
  margin: 0;
  padding-top: 0.25rem;
  color: var(--text-muted);
  font-size: 0.72rem;
  font-style: italic;
}
</style>
