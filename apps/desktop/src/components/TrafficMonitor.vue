<script setup lang="ts">
import { nextTick, onUnmounted, ref, watch } from 'vue';
import TrafficEntry from './TrafficEntry.vue';
import { useTrafficStore } from '../stores/traffic';

const emit = defineEmits<{
  close: [];
  resize: [height: number];
}>();

const trafficStore = useTrafficStore();
const logContainer = ref<HTMLElement | null>(null);
const autoScroll = ref(true);
const isResizing = ref(false);
const panelHeight = ref(220);

const MIN_HEIGHT = 100;
const MAX_HEIGHT_RATIO = 0.7;

function startResize(event: MouseEvent) {
  isResizing.value = true;
  event.preventDefault();
  document.addEventListener('mousemove', doResize);
  document.addEventListener('mouseup', stopResize);
  document.body.style.cursor = 'ns-resize';
  document.body.style.userSelect = 'none';
}

function doResize(event: MouseEvent) {
  if (!isResizing.value) return;

  const maxHeight = window.innerHeight * MAX_HEIGHT_RATIO;
  const newHeight = window.innerHeight - event.clientY;
  panelHeight.value = Math.min(maxHeight, Math.max(MIN_HEIGHT, newHeight));
  emit('resize', panelHeight.value);
}

function stopResize() {
  isResizing.value = false;
  document.removeEventListener('mousemove', doResize);
  document.removeEventListener('mouseup', stopResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

function handleScroll() {
  if (!logContainer.value) return;
  const { scrollTop, scrollHeight, clientHeight } = logContainer.value;
  autoScroll.value = scrollTop + clientHeight >= scrollHeight - 50;
}

watch(
  () => trafficStore.entries.length,
  async () => {
    if (!autoScroll.value || !logContainer.value || trafficStore.isPaused) return;
    await nextTick();
    logContainer.value.scrollTop = logContainer.value.scrollHeight;
  },
);

onUnmounted(stopResize);
</script>

<template>
  <div class="traffic-monitor" :style="{ height: `${panelHeight}px` }">
    <div class="resize-handle" title="Drag to resize" @mousedown="startResize"></div>

    <div class="monitor-header">
      <span class="title">📡 ACP Traffic</span>

      <div class="controls">
        <button
          class="control-btn"
          :class="{ active: trafficStore.isPaused }"
          :title="trafficStore.isPaused ? 'Resume' : 'Pause'"
          @click="trafficStore.togglePause()"
        >
          {{ trafficStore.isPaused ? '▶' : '⏸' }}
        </button>

        <button class="control-btn" title="Clear" @click="trafficStore.clear()">🗑</button>

        <div class="search-container">
          <span class="search-icon">🔍</span>
          <input
            type="text"
            class="search-input"
            placeholder="Search..."
            :value="trafficStore.searchQuery"
            @input="trafficStore.setSearch(($event.target as HTMLInputElement).value)"
          />
          <button
            v-if="trafficStore.searchQuery"
            class="search-clear-btn"
            title="Clear search"
            @click="trafficStore.clearSearch()"
          >
            ×
          </button>
        </div>

        <span v-if="trafficStore.searchQuery" class="match-count">
          {{ trafficStore.filteredEntries.length }} match{{ trafficStore.filteredEntries.length === 1 ? '' : 'es' }}
        </span>

        <select
          class="filter-select"
          :value="trafficStore.filter"
          @change="trafficStore.setFilter(($event.target as HTMLSelectElement).value as typeof trafficStore.filter)"
        >
          <option value="all">All</option>
          <option value="requests">Requests</option>
          <option value="responses">Responses</option>
          <option value="notifications">Notifications</option>
        </select>
      </div>

      <button class="close-btn" title="Close" @click="emit('close')">×</button>
    </div>

    <div ref="logContainer" class="log-container" @scroll="handleScroll">
      <div v-if="trafficStore.filteredEntries.length === 0" class="empty-state">
        No traffic captured yet. Connect to an agent to see ACP messages.
      </div>

      <TrafficEntry
        v-for="entry in trafficStore.filteredEntries"
        :key="entry.id"
        :entry="entry"
      />
    </div>

    <div v-if="trafficStore.isPaused" class="paused-indicator">⏸ Paused</div>
  </div>
</template>

<style scoped>
.traffic-monitor {
  display: flex;
  flex-direction: column;
  position: relative;
  background: var(--bg-panel);
  border-top: 1px solid var(--border-default);
  font-size: var(--text-caption);
}

.resize-handle {
  position: absolute;
  inset: 0 0 auto;
  height: 6px;
  cursor: ns-resize;
  background: transparent;
  z-index: 10;
}

.resize-handle:hover,
.resize-handle:active {
  background: var(--border-focus);
  opacity: 0.5;
}

.monitor-header,
.controls {
  display: flex;
  align-items: center;
}

.monitor-header {
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: var(--bg-main);
  border-bottom: 1px solid var(--border-default);
}

.title {
  margin-right: auto;
  font-weight: var(--weight-title);
}

.controls {
  gap: 0.25rem;
}

.control-btn,
.filter-select {
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--border-default);
  border-radius: 4px;
  font-size: var(--text-tiny);
  cursor: pointer;
}

.control-btn {
  background: transparent;
}

.control-btn:hover,
.close-btn:hover {
  background: var(--bg-surface);
}

.control-btn.active,
.paused-indicator {
  background: var(--bg-warning);
}

.search-container {
  position: relative;
  display: flex;
  align-items: center;
  padding: 0 0.25rem;
  background: var(--bg-main);
  border: 1px solid var(--border-default);
  border-radius: 4px;
}

.search-icon,
.match-count,
.close-btn {
  color: var(--text-muted);
}

.search-icon {
  font-size: var(--text-tiny);
  opacity: 0.6;
}

.search-input {
  width: 120px;
  padding: 0.25rem 0.4rem;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: var(--text-tiny);
  outline: none;
}

.search-input::placeholder {
  color: var(--text-muted);
}

.search-clear-btn,
.close-btn {
  border: none;
  background: transparent;
  cursor: pointer;
}

.search-clear-btn {
  padding: 0 0.25rem;
  color: var(--text-muted);
  font-size: var(--text-body-sm);
  line-height: 1;
}

.search-clear-btn:hover,
.close-btn:hover {
  color: var(--text-primary);
}

.match-count {
  font-size: var(--text-tiny);
  white-space: nowrap;
}

.filter-select {
  background: var(--bg-main);
  color: var(--text-primary);
}

.close-btn {
  padding: 0.25rem 0.5rem;
  font-size: var(--text-body-sm);
}

.log-container {
  flex: 1;
  overflow-y: auto;
  font-family: var(--font-code);
}

.empty-state {
  padding: 2rem;
  text-align: center;
  color: var(--text-muted);
}

.paused-indicator {
  position: absolute;
  right: 0.5rem;
  bottom: 0.5rem;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: var(--text-tiny);
  color: color-mix(in srgb, var(--color-status-stuck) 72%, var(--text-primary));
}
</style>
