<script setup lang="ts">
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

const controlBtnClass = 'py-1 px-2 rounded-sm bg-surface-container text-[var(--text)] text-tiny hover:bg-surface-high';
const filterPillBase = 'py-[0.2rem] px-2 rounded-sm text-tiny transition-colors duration-150';
</script>

<template>
  <!-- audit-ok: dynamic-binding panel height is set from JS via :style for user resizing. -->
  <section class="flex flex-col relative bg-surface text-[length:var(--text-caption)]" :style="{ height: `${panelHeight}px` }">
    <div
      class="absolute top-0 inset-x-0 h-1.5 cursor-ns-resize z-10 hover:bg-brand hover:opacity-50 active:bg-brand active:opacity-50"
      title="Drag to resize"
      @mousedown="startResize"
    />

    <div class="flex items-center gap-2 py-2 px-3 bg-bg-app">
      <span class="font-display font-medium text-sm text-[var(--text)]">ACP traffic</span>

      <div class="flex items-center gap-1 ml-auto">
        <Button
          variant="ghost"
          size="xs"
          :class="[controlBtnClass, trafficStore.isPaused ? 'bg-warning' : '']"
          :title="trafficStore.isPaused ? 'Resume' : 'Pause'"
          @click="trafficStore.togglePause()"
        >
          {{ trafficStore.isPaused ? '▶' : '⏸' }}
        </Button>
        <Button variant="ghost" size="xs" :class="controlBtnClass" title="Clear" @click="trafficStore.clear()">🗑</Button>

        <div class="flex gap-1 ml-2">
          <Button
            v-for="opt in filterOptions"
            :key="opt.value"
            variant="ghost"
            size="xs"
            :class="[
              filterPillBase,
              trafficStore.filter === opt.value
                ? 'bg-brand text-[var(--text)]'
                : 'bg-surface-container text-text-text-secondary hover:bg-surface-high',
            ]"
            @click="trafficStore.setFilter(opt.value)"
          >
            {{ opt.label }}
          </Button>
        </div>

        <div class="flex gap-1 ml-2">
          <Button
            v-for="opt in directionOptions"
            :key="opt.value"
            variant="ghost"
            size="xs"
            :class="[
              filterPillBase,
              trafficStore.directionFilter === opt.value
                ? 'bg-brand text-[var(--text)]'
                : 'bg-surface-container text-text-text-secondary hover:bg-surface-high',
            ]"
            @click="trafficStore.setDirectionFilter(opt.value)"
          >
            {{ opt.label }}
          </Button>
        </div>
      </div>

      <div class="flex items-center ml-2 bg-surface rounded-sm">
        <Input
          type="text"
          class="w-[120px] py-1 px-1.5 bg-transparent text-[var(--text)] text-tiny border-0 shadow-none ring-0 focus-visible:ring-0 focus-visible:border-transparent focus:outline-none placeholder:text-text-text-secondary h-auto rounded-sm"
          placeholder="Filter method or notification…"
          :model-value="trafficStore.methodFilter"
          @update:model-value="(value) => trafficStore.setMethodFilter(String(value))"
        />
        <Button
          v-if="trafficStore.methodFilter"
          variant="ghost"
          size="xs"
          class="py-0 px-1 bg-transparent text-text-text-secondary text-[length:var(--text-body-sm)] leading-none hover:text-[var(--text)]"
          title="Clear search"
          @click="trafficStore.clearMethodFilter()"
        >
          ×
        </Button>
      </div>

      <span v-if="trafficStore.methodFilter" class="text-text-text-secondary text-tiny whitespace-nowrap">
        {{ trafficStore.filteredEntries.length }} match{{ trafficStore.filteredEntries.length === 1 ? '' : 'es' }}
      </span>

      <Button
        variant="ghost"
        size="xs"
        class="py-1 px-2 bg-transparent text-text-text-secondary text-[length:var(--text-body-sm)] hover:bg-surface-high"
        title="Close"
        @click="emit('close')"
      >×</Button>
    </div>

    <div
      ref="logContainer"
      class="flex-1 overflow-y-auto font-mono"
      role="log"
      aria-live="polite"
      aria-label="Traffic monitor"
      @scroll="handleScroll"
    >
      <div v-if="trafficStore.filteredEntries.length === 0" class="p-8 text-center text-text-text-secondary">
        No traffic captured yet. Connect to an agent to see ACP messages.
      </div>
      <TrafficEntry
        v-for="entry in trafficStore.filteredEntries"
        :key="entry.id"
        :entry="entry"
      />
    </div>

    <!-- audit-ok: dynamic-binding inline color-mix() blends status-stuck with text color for paused indicator label tone. -->
    <div
      v-if="trafficStore.isPaused"
      class="absolute right-2 bottom-2 py-1 px-2 rounded-sm bg-warning text-tiny"
      style="color: color-mix(in srgb, var(--color-status-stuck) 72%, var(--text));"
    >⏸ Paused</div>
  </section>
</template>
