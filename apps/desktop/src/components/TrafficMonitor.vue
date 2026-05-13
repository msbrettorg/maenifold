<script setup lang="ts">
import { nextTick, onUnmounted, ref, watch } from 'vue';
import { Pause, Play, Trash2, X } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import TrafficEntry from './TrafficEntry.vue';
import { useTrafficStore } from '../stores/traffic';

const emit = defineEmits<{ close: []; resize: [height: number] }>();

const trafficStore = useTrafficStore();
const scrollAreaRef = ref<InstanceType<typeof ScrollArea> | null>(null);
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

function getViewport(): HTMLElement | null {
  const root = scrollAreaRef.value as unknown as { $el?: HTMLElement } | null;
  const host = root?.$el ?? null;
  if (!host) return null;
  return host.querySelector('[data-slot="scroll-area-viewport"]') as HTMLElement | null;
}

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
  const viewport = getViewport();
  if (!viewport) return;
  const { scrollTop, scrollHeight, clientHeight } = viewport;
  autoScroll.value = scrollTop + clientHeight >= scrollHeight - 50;
}

watch(
  () => trafficStore.entries.length,
  async () => {
    if (!autoScroll.value || trafficStore.isPaused) return;
    await nextTick();
    const viewport = getViewport();
    if (!viewport) return;
    viewport.scrollTop = viewport.scrollHeight;
  },
);

onUnmounted(stopResize);
</script>

<template>
  <section
    class="relative flex flex-col bg-surface text-text-primary text-tiny"
    :style="{ height: `${panelHeight}px` }"
  >
    <div
      class="absolute inset-x-0 top-0 z-10 h-1.5 cursor-ns-resize hover:bg-brand/50 active:bg-brand/50"
      title="Drag to resize"
      @mousedown="startResize"
    />

    <div class="flex flex-wrap items-center gap-2 gap-y-2 bg-bg-app px-3 py-2">
      <span class="text-sm font-medium text-text-primary">ACP traffic</span>

      <div class="ml-auto flex items-center gap-1">
        <Button
          variant="ghost"
          size="xs"
          :class="['gap-1 px-2 py-1 text-tiny', trafficStore.isPaused ? 'bg-warning text-text-on-accent hover:bg-warning' : '']"
          :title="trafficStore.isPaused ? 'Resume' : 'Pause'"
          @click="trafficStore.togglePause()"
        >
          <Play v-if="trafficStore.isPaused" class="size-3" />
          <Pause v-else class="size-3" />
        </Button>
        <Button
          variant="ghost"
          size="xs"
          class="gap-1 px-2 py-1 text-tiny"
          title="Clear"
          @click="trafficStore.clear()"
        >
          <Trash2 class="size-3" />
        </Button>

        <div class="ml-2 flex gap-1">
          <Button
            v-for="opt in filterOptions"
            :key="opt.value"
            variant="ghost"
            size="xs"
            :class="[
              'px-2 py-0.5 text-tiny transition-colors duration-150',
              trafficStore.filter === opt.value
                ? 'bg-brand text-text-on-accent hover:bg-brand'
                : 'text-text-secondary hover:bg-surface-high',
            ]"
            @click="trafficStore.setFilter(opt.value)"
          >
            {{ opt.label }}
          </Button>
        </div>

        <div class="ml-2 flex gap-1">
          <Button
            v-for="opt in directionOptions"
            :key="opt.value"
            variant="ghost"
            size="xs"
            :class="[
              'px-2 py-0.5 text-tiny transition-colors duration-150',
              trafficStore.directionFilter === opt.value
                ? 'bg-brand text-text-on-accent hover:bg-brand'
                : 'text-text-secondary hover:bg-surface-high',
            ]"
            @click="trafficStore.setDirectionFilter(opt.value)"
          >
            {{ opt.label }}
          </Button>
        </div>
      </div>

      <div class="ml-2 flex items-center gap-1 rounded-sm bg-surface">
        <Input
          type="text"
          class="h-auto w-[140px] rounded-sm border-0 bg-transparent px-1.5 py-1 text-tiny text-text-primary shadow-none ring-0 placeholder:text-text-secondary focus:outline-none focus-visible:border-transparent focus-visible:ring-0"
          placeholder="Filter method or notification…"
          :model-value="trafficStore.methodFilter"
          @update:model-value="(value) => trafficStore.setMethodFilter(String(value))"
        />
        <Button
          v-if="trafficStore.methodFilter"
          variant="ghost"
          size="xs"
          class="px-1 py-0 text-text-secondary hover:bg-surface-high hover:text-text-primary"
          title="Clear search"
          @click="trafficStore.clearMethodFilter()"
        >
          <X class="size-3" />
        </Button>
      </div>

      <span
        v-if="trafficStore.methodFilter"
        class="whitespace-nowrap text-tiny text-text-secondary"
      >
        {{ trafficStore.filteredEntries.length }} match{{ trafficStore.filteredEntries.length === 1 ? '' : 'es' }}
      </span>

      <Button
        variant="ghost"
        size="xs"
        class="px-2 py-1 text-text-secondary hover:bg-surface-high hover:text-text-primary"
        title="Close"
        @click="emit('close')"
      >
        <X class="size-3.5" />
      </Button>
    </div>

    <ScrollArea
      ref="scrollAreaRef"
      class="flex-1 font-mono"
      role="log"
      aria-live="polite"
      aria-label="Traffic monitor"
      @scroll.passive="handleScroll"
    >
      <div
        v-if="trafficStore.filteredEntries.length === 0"
        class="p-8 text-center text-text-secondary"
      >
        No traffic captured yet. Connect to an agent to see ACP messages.
      </div>
      <TrafficEntry
        v-for="entry in trafficStore.filteredEntries"
        :key="entry.id"
        :entry="entry"
      />
    </ScrollArea>

    <Badge
      v-if="trafficStore.isPaused"
      variant="outline"
      class="absolute bottom-2 right-2 gap-1 bg-warning text-text-on-accent"
    >
      <Pause class="size-3" />
      Paused
    </Badge>
  </section>
</template>
