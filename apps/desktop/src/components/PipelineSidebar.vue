<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { computed, onMounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import type { PipelineListItem, PipelineStatus } from '../lib/pipeline-types';

const props = withDefaults(defineProps<{
  hideHeader?: boolean;
  filterQuery?: string;
}>(), {
  hideHeader: false,
  filterQuery: '',
});

const emit = defineEmits<{
  (e: 'pipeline-selected', pipelineId: string): void;
}>();

const store = usePipelineStore();
const { connected, pipelines, currentPipelineId, hasDirtyDraft } = storeToRefs(store);

const pipelineItems = computed(() => {
  const q = props.filterQuery.trim().toLowerCase();
  if (!q) return pipelines.value;
  return pipelines.value.filter((p) => p.pipelineId.toLowerCase().includes(q));
});

onMounted(async () => {
  if (connected.value) await store.refreshList();
});

watch(connected, async (c) => {
  if (c) await store.refreshList();
});

async function handleRefresh() {
  if (connected.value) await store.refreshList();
}

function formatStatus(status: PipelineStatus): string {
  return status.charAt(0).toUpperCase() + status.slice(1);
}

function progressPct(item: PipelineListItem): number {
  return item.nodeCount > 0 ? (item.completedCount / item.nodeCount) * 100 : 0;
}

async function handleSelect(pipelineId: string) {
  if (currentPipelineId.value === pipelineId) return;
  if (hasDirtyDraft.value && !window.confirm('Discard unsaved pipeline draft?')) return;
  const loaded = await store.loadPipeline(pipelineId);
  if (loaded) emit('pipeline-selected', pipelineId);
}

function handleNew() {
  if (hasDirtyDraft.value && !window.confirm('Discard unsaved pipeline draft?')) return;
  store.newPipeline();
}

async function handleDelete(item: PipelineListItem, event: Event) {
  event.stopPropagation();
  if (!confirm(`Delete pipeline "${item.pipelineId}"?`)) return;
  await store.deletePipelineById(item.pipelineId);
}
</script>

<template>
  <aside class="pipeline-sidebar" role="complementary" aria-label="Pipeline sidebar">
    <div v-if="!hideHeader" class="sidebar-header">
      <h3>Pipelines</h3>
      <div class="sidebar-actions">
        <Button variant="ghost" size="sm" type="button" title="Refresh" @click="handleRefresh">↻</Button>
        <Button variant="ghost" size="sm" type="button" @click="handleNew">+ New</Button>
      </div>
    </div>

    <div v-if="pipelineItems.length === 0" class="empty-state">
      <p>No pipelines yet</p>
    </div>

    <ScrollArea v-else class="pipeline-list-scroll">
      <ul class="pipeline-list">
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
            <span :class="['status-bead', `bead--${item.status}`]" />

            <div class="pipeline-meta">
              <span class="pipeline-id" :title="item.pipelineId">{{ item.pipelineId }}</span>
              <span class="pipeline-progress">{{ item.completedCount }}/{{ item.nodeCount }} nodes</span>
              <div class="progress-bar">
                <div class="progress-fill" :style="{ width: `${progressPct(item)}%` }" />
              </div>
            </div>
          </button>

          <div class="pipeline-side">
            <span :class="['pipeline-status', `status-text--${item.status}`]">
              {{ formatStatus(item.status) }}
            </span>
            <Button
              class="delete-btn h-6 rounded-[4px] px-2 text-xs"
              variant="destructive"
              size="sm"
              type="button"
              title="Delete pipeline"
              aria-label="Delete pipeline"
              @click="handleDelete(item, $event)"
            >×</Button>
          </div>
        </li>
      </ul>
    </ScrollArea>
  </aside>
</template>

<style scoped>
.pipeline-sidebar {
  width: 100%;
  min-width: 0;
  height: 100%;
  background: var(--surface);
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
}

.sidebar-header h3 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
  font-size: var(--text-title);
  color: var(--color-text-primary);
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
  color: var(--text-dim);
}

.pipeline-list-scroll {
  flex: 1;
  min-height: 0;
}

.pipeline-list {
  list-style: none;
  margin: 0;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.pipeline-item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem;
  border-left: 4px solid transparent;
  border-radius: 4px;
  background: var(--surface-container);
  transition: background 0.15s, border-color 0.15s;
}

.pipeline-item:hover { background: var(--surface-high); }

.pipeline-item.active {
  border-left-color: var(--blue);
  background: var(--surface-high);
}

.pipeline-select {
  min-width: 0;
  flex: 1;
  padding: 0;
  border: 0;
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.pipeline-select:focus-visible {
  outline: 2px solid var(--color-brand);
  outline-offset: 3px;
  border-radius: 4px;
}

.status-bead {
  width: 6px;
  height: 6px;
  border-radius: var(--radius);
  flex-shrink: 0;
  margin-top: 0.375rem;
}

.bead--pending { background: var(--text-dim); }
.bead--active { background: var(--amber); }
.bead--completed { background: var(--sage); }
.bead--abandoned, .bead--stuck { background: var(--terracotta); }
.bead--ready, .bead--paused { background: var(--sage); }
.bead--cancelled { background: var(--text-dim); }

.pipeline-meta {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.pipeline-id {
  font-weight: var(--weight-display);
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pipeline-progress {
  font-size: var(--text-xs);
  color: var(--text-dim);
}

.progress-bar {
  height: 3px;
  border-radius: 2px;
  background: var(--color-bg-app);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 2px;
  background: var(--blue);
  transition: width 0.3s ease;
}

.pipeline-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.375rem;
  min-width: 0;
}

.pipeline-status {
  font-size: var(--text-xs);
  font-weight: var(--weight-display);
}

.status-text--pending { color: var(--color-status-pending); }
.status-text--active { color: var(--color-status-active); }
.status-text--completed { color: var(--color-status-completed); }
.status-text--abandoned { color: var(--color-status-abandoned); }
.status-text--stuck { color: var(--color-status-stuck); }
.status-text--ready, .status-text--paused { color: var(--color-status-ready); }
.status-text--cancelled { color: var(--color-status-abandoned); }

.delete-btn {
  line-height: 1;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s;
}

.pipeline-item:hover .delete-btn,
.pipeline-item:focus-within .delete-btn {
  opacity: 1;
}
</style>
