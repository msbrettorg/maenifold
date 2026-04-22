<script setup lang="ts"> // audit-ok: max-loc
import { nextTick, onUnmounted, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import TrafficEntry from './TrafficEntry.vue';
import { useTrafficStore } from '../stores/traffic';

const emit = defineEmits<{ close: []; resize: [height: number] }>();

const trafficStore = useTrafficStore();
const logContainer = ref<HTMLElement | null>(null);
const autoScroll = ref(true);
const isResizing = ref(false);
const panelHeight = ref(220);
const MIN_HEIGHT = 100;
const MAX_HEIGHT_RATIO = 0.7;

type FilterValue = typeof trafficStore.filter;
type DirectionValue = typeof trafficStore.directionFilter;
const filterOptions: { value: FilterValue; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'requests', label: 'Requests' },
  { value: 'responses', label: 'Responses' },
  { value: 'notifications', label: 'Notifications' },
];
const directionOptions: { value: DirectionValue; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'outbound', label: 'Outbound' },
  { value: 'inbound', label: 'Inbound' },
];

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
  panelHeight.value = Math.min(maxHeight, Math.max(MIN_HEIGHT, window.innerHeight - event.clientY));
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
  <section class="traffic-monitor" :style="{ height: `${panelHeight}px` }">
    <div class="resize-handle" title="Drag to resize" @mousedown="startResize" />

    <div class="monitor-header">
      <span class="title">ACP traffic</span>

      <div class="controls">
        <Button
          variant="ghost"
          size="xs"
          class="control-btn"
          :class="{ active: trafficStore.isPaused }"
          :title="trafficStore.isPaused ? 'Resume' : 'Pause'"
          @click="trafficStore.togglePause()"
        >
          {{ trafficStore.isPaused ? '▶' : '⏸' }}
        </Button>
        <Button variant="ghost" size="xs" class="control-btn" title="Clear" @click="trafficStore.clear()">🗑</Button>

        <div class="filter-pills">
          <Button
            v-for="opt in filterOptions"
            :key="opt.value"
            variant="ghost"
            size="xs"
            :class="['filter-pill', { active: trafficStore.filter === opt.value }]"
            @click="trafficStore.setFilter(opt.value)"
          >
            {{ opt.label }}
          </Button>
        </div>

        <div class="filter-pills">
          <Button
            v-for="opt in directionOptions"
            :key="opt.value"
            variant="ghost"
            size="xs"
            :class="['filter-pill', { active: trafficStore.directionFilter === opt.value }]"
            @click="trafficStore.setDirectionFilter(opt.value)"
          >
            {{ opt.label }}
          </Button>
        </div>
      </div>

      <div class="search-container">
        <Input
          type="text"
          class="search-input"
          placeholder="Filter method or notification…"
          :model-value="trafficStore.methodFilter"
          @update:model-value="(value) => trafficStore.setMethodFilter(String(value))"
        />
        <Button
          v-if="trafficStore.methodFilter"
          variant="ghost"
          size="xs"
          class="search-clear-btn"
          title="Clear search"
          @click="trafficStore.clearMethodFilter()"
        >
          ×
        </Button>
      </div>

      <span v-if="trafficStore.methodFilter" class="match-count">
        {{ trafficStore.filteredEntries.length }} match{{ trafficStore.filteredEntries.length === 1 ? '' : 'es' }}
      </span>

      <Button variant="ghost" size="xs" class="close-btn" title="Close" @click="emit('close')">×</Button>
    </div>

    <div
      ref="logContainer"
      class="log-container"
      role="log"
      aria-live="polite"
      aria-label="Traffic monitor"
      @scroll="handleScroll"
    >
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
  </section>
</template>

<!-- audit-ok: max-style-rules -->
<style scoped>
.traffic-monitor {
  display: flex;
  flex-direction: column;
  position: relative;
  background: var(--surface);
  font-size: var(--text-caption);
}
.resize-handle {
  position: absolute;
  inset: 0 0 auto;
  height: 6px;
  cursor: ns-resize;
  z-index: 10;
}
.resize-handle:hover,
.resize-handle:active { background: var(--color-brand); opacity: 0.5; }
.monitor-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: var(--color-bg-app);
}
.title {
  font-family: var(--font-display);
  font-weight: 500;
  font-size: var(--text-sm);
  color: var(--text);
}
.controls {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  margin-left: auto;
}
.control-btn {
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: 4px;
  background: var(--surface-container);
  color: var(--text);
  font-size: var(--text-tiny);
  cursor: pointer;
}
.control-btn:hover,
.close-btn:hover { background: var(--surface-high); }
.control-btn.active { background: var(--bg-warning); }
.filter-pills {
  display: flex;
  gap: 0.25rem;
  margin-left: 0.5rem;
}
.filter-pill {
  padding: 0.2rem 0.5rem;
  border: none;
  border-radius: 4px;
  background: var(--surface-container);
  color: var(--text-secondary);
  font-size: var(--text-tiny);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.filter-pill:hover { background: var(--surface-high); }
.filter-pill.active {
  background: var(--color-brand);
  border-color: var(--color-brand);
  color: var(--text);
}
.search-container {
  display: flex;
  align-items: center;
  margin-left: 0.5rem;
  background: var(--surface);
  border-radius: 4px;
}
.search-input {
  width: 120px;
  padding: 0.25rem 0.4rem;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: var(--text-tiny);
  outline: none;
}
.search-input::placeholder { color: var(--text-secondary); }
.search-clear-btn {
  padding: 0 0.25rem;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--text-body-sm);
  line-height: 1;
  cursor: pointer;
}
.search-clear-btn:hover { color: var(--text); }
.match-count {
  color: var(--text-secondary);
  font-size: var(--text-tiny);
  white-space: nowrap;
}
.close-btn {
  padding: 0.25rem 0.5rem;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--text-body-sm);
  cursor: pointer;
}
.log-container {
  flex: 1;
  overflow-y: auto;
  font-family: var(--font-code);
}
.empty-state {
  padding: 2rem;
  text-align: center;
  color: var(--text-secondary);
}
.paused-indicator {
  position: absolute;
  right: 0.5rem;
  bottom: 0.5rem;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  background: var(--bg-warning);
  font-size: var(--text-tiny);
  color: color-mix(in srgb, var(--color-status-stuck) 72%, var(--text));
}
</style>
