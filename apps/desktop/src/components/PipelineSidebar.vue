<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import type { PipelineListItem, PipelineStatus } from '../lib/pipeline-types';

withDefaults(defineProps<{
  hideHeader?: boolean;
}>(), {
  hideHeader: false,
});

const emit = defineEmits<{
  (e: 'pipeline-selected', pipelineId: string): void;
}>();

const store = usePipelineStore();
const { connected, pipelines, currentPipelineId, hasDirtyDraft } = storeToRefs(store);

const STATUS_ICONS: Record<string, string> = {
  active: '🔄',
  paused: '⏸',
  completed: '✅',
  cancelled: '⏹',
  stuck: '🚫',
};

const pipelineItems = computed(() => pipelines.value);

onMounted(async () => {
  if (connected.value) {
    await store.refreshList();
  }
});

watch(connected, async (isConnected) => {
  if (isConnected) {
    await store.refreshList();
  }
});

async function handleRefresh() {
  if (connected.value) {
    await store.refreshList();
  }
}

function getStatusIcon(status: PipelineStatus): string {
  return STATUS_ICONS[status] ?? '⏳';
}

function formatStatus(status: PipelineStatus): string {
  return status.charAt(0).toUpperCase() + status.slice(1);
}

async function handleSelect(pipelineId: string) {
  if (currentPipelineId.value === pipelineId) {
    return;
  }

  if (hasDirtyDraft.value && !window.confirm('Discard unsaved pipeline draft?')) {
    return;
  }

  const loaded = await store.loadPipeline(pipelineId);
  if (!loaded) {
    return;
  }

  emit('pipeline-selected', pipelineId);
}

function handleNew() {
  if (hasDirtyDraft.value && !window.confirm('Discard unsaved pipeline draft?')) {
    return;
  }
  store.newPipeline();
}

async function handleDelete(item: PipelineListItem, event: Event) {
  event.stopPropagation();

  if (!confirm(`Delete pipeline "${item.pipelineId}"?`)) {
    return;
  }

  await store.deletePipelineById(item.pipelineId);
}
</script>

<template>
  <aside class="pipeline-sidebar">
    <div v-if="!hideHeader" class="sidebar-header">
      <h3>Pipelines</h3>
      <div class="sidebar-actions">
        <button class="btn btn-ghost" type="button" title="Refresh" @click="handleRefresh">↻</button>
        <button class="btn btn-ghost" type="button" @click="handleNew">+ New</button>
      </div>
    </div>

    <div v-if="pipelineItems.length === 0" class="empty-state">
      <p>No pipelines yet</p>
    </div>

    <ul v-else class="pipeline-list">
      <li
        v-for="item in pipelineItems"
        :key="item.pipelineId"
        :class="['pipeline-item', { active: currentPipelineId === item.pipelineId }]"
      >
        <button
          type="button"
          class="pipeline-select"
          :data-pipeline-id="item.pipelineId"
          data-pipeline-item
          :aria-current="currentPipelineId === item.pipelineId ? 'page' : undefined"
          @click="handleSelect(item.pipelineId)"
        >
          <div class="pipeline-main">
            <span class="status-icon" :aria-label="`${item.status} pipeline`">
              {{ getStatusIcon(item.status) }}
            </span>

            <div class="pipeline-meta">
              <span class="pipeline-id" :title="item.pipelineId">{{ item.pipelineId }}</span>
              <span class="pipeline-progress">{{ item.completedCount }}/{{ item.nodeCount }} nodes</span>
            </div>
          </div>
        </button>

        <div class="pipeline-side">
          <span :class="['pipeline-status', 'status-text', `status-text--${item.status}`]">
            {{ formatStatus(item.status) }}
          </span>
          <button
            class="btn btn-outline-danger delete-btn"
            type="button"
            title="Delete pipeline"
            aria-label="Delete pipeline"
            @click="handleDelete(item, $event)"
          >
            ×
          </button>
        </div>
      </li>
    </ul>
  </aside>
</template>

<style scoped>
.pipeline-sidebar {
  width: 100%;
  min-width: 0;
  height: 100%;
  border-right: 1px solid var(--border-default);
  background: var(--bg-panel);
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
  border-bottom: 1px solid var(--border-default);
}

.sidebar-header h3 {
  margin: 0;
  font-size: var(--text-xl);
  color: var(--text-secondary);
}

.sidebar-actions {
  display: flex;
  gap: 0.25rem;
}

.empty-state {
  flex: 1;
  display: grid;
  place-items: center;
  padding: 2rem 1rem;
  text-align: center;
  color: var(--text-muted);
}

.pipeline-list {
  list-style: none;
  margin: 0;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  overflow-y: auto;
}

.pipeline-item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-left: 3px solid transparent;
  border-radius: 6px;
  background: var(--bg-main);
  transition: background 0.15s ease, border-color 0.15s ease;
}

.pipeline-item:hover {
  background: var(--bg-surface);
}

.pipeline-item.active {
  border-left-color: var(--border-focus);
  background: var(--bg-surface);
}

.pipeline-select {
  min-width: 0;
  flex: 1;
  padding: 0;
  border: 0;
  display: flex;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.pipeline-select:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 3px;
  border-radius: 4px;
}

.pipeline-main {
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  min-width: 0;
  flex: 1;
}

.status-icon {
  line-height: 1.2;
}

.pipeline-meta,
.pipeline-side {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.pipeline-meta {
  flex: 1;
  gap: 0.25rem;
}

.pipeline-id {
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pipeline-progress {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.pipeline-side {
  align-items: flex-end;
  gap: 0.375rem;
}

.pipeline-status {
  font-size: var(--text-xs);
  font-weight: 500;
}

.delete-btn {
  line-height: 1;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.pipeline-item:hover .delete-btn,
.pipeline-item:focus-within .delete-btn {
  opacity: 1;
}

</style>
