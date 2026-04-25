<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import GateTaskDetail from './GateTaskDetail.vue';
import GateTaskTable from './GateTaskTable.vue';
import type { GateTask, GateTaskHistoryEntry } from '../lib/gate';

type SortColumn = 'id' | 'name' | 'process' | 'progress';
type SortDirection = 'asc' | 'desc';

const GATE_SPLITTER_RATIO_KEY = 'gate-splitter-ratio';
const DEFAULT_LEFT_RATIO = 0.45;
const MIN_LEFT_WIDTH = 300;
const MIN_RIGHT_WIDTH = 400;
const MIN_TOTAL_WIDTH = MIN_LEFT_WIDTH + MIN_RIGHT_WIDTH;
const FALLBACK_CONTAINER_WIDTH = 1200;

defineProps<{
  tasks: GateTask[];
  selectedTaskKey: string;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
  activeTask: GateTask | null;
  body: string;
  history: GateTaskHistoryEntry[];
  bodyLoading: boolean;
  historyLoading: boolean;
  bodyError: string;
  historyError: string;
}>();

defineEmits<{
  sort: [column: SortColumn];
  select: [task: GateTask];
  close: [];
}>();

const containerEl = ref<HTMLElement | null>(null);
const containerWidth = ref(FALLBACK_CONTAINER_WIDTH);
const storedLeftRatio = ref(readStoredRatio());

let resizeObserver: ResizeObserver | null = null;

const effectiveContainerWidth = computed(() =>
  Math.max(containerWidth.value, MIN_TOTAL_WIDTH)
);
const minLeftSize = computed(() => (MIN_LEFT_WIDTH / effectiveContainerWidth.value) * 100);
const minRightSize = computed(() => (MIN_RIGHT_WIDTH / effectiveContainerWidth.value) * 100);
const maxLeftSize = computed(() => 100 - minRightSize.value);
const defaultLeftSize = computed(() =>
  clamp(storedLeftRatio.value * 100, minLeftSize.value, maxLeftSize.value)
);
const defaultRightSize = computed(() => 100 - defaultLeftSize.value);

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function readStoredRatio() {
  if (typeof window === 'undefined') {
    return DEFAULT_LEFT_RATIO;
  }

  const raw = window.localStorage.getItem(GATE_SPLITTER_RATIO_KEY);
  if (raw === null) {
    return DEFAULT_LEFT_RATIO;
  }

  try {
    const parsed = JSON.parse(raw) as number;
    return Number.isFinite(parsed) ? clamp(parsed, 0, 1) : DEFAULT_LEFT_RATIO;
  } catch {
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? clamp(parsed, 0, 1) : DEFAULT_LEFT_RATIO;
  }
}

function writeStoredRatio(value: number) {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(GATE_SPLITTER_RATIO_KEY, JSON.stringify(clamp(value, 0, 1)));
}

function updateContainerWidth() {
  const nextWidth = containerEl.value?.clientWidth ?? 0;
  if (nextWidth > 0) {
    containerWidth.value = nextWidth;
  }
}

function handleLayout(layout: number[]) {
  const [leftSize] = layout;
  if (typeof leftSize !== 'number' || Number.isNaN(leftSize)) {
    return;
  }

  const nextRatio = clamp(leftSize / 100, 0, 1);
  storedLeftRatio.value = nextRatio;
  writeStoredRatio(nextRatio);
}

onMounted(() => {
  updateContainerWidth();
  if (typeof ResizeObserver === 'undefined' || !containerEl.value) {
    return;
  }

  resizeObserver = new ResizeObserver(() => {
    updateContainerWidth();
  });
  resizeObserver.observe(containerEl.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <div ref="containerEl" class="flex flex-1 min-h-0">
    <ResizablePanelGroup
      direction="horizontal"
      class="h-full w-full"
      @layout="handleLayout"
    >
      <ResizablePanel
        id="gate-task-table"
        :default-size="defaultLeftSize"
        :min-size="minLeftSize"
        :max-size="maxLeftSize"
        :order="1"
      >
        <div class="flex h-full min-h-0 min-w-0">
          <GateTaskTable
            :tasks="tasks"
            :selected-task-key="selectedTaskKey"
            :sort-column="sortColumn"
            :sort-direction="sortDirection"
            @sort="$emit('sort', $event)"
            @select="$emit('select', $event)"
          />
        </div>
      </ResizablePanel>
      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />
      <ResizablePanel
        id="gate-task-detail"
        :default-size="defaultRightSize"
        :min-size="minRightSize"
        :order="2"
      >
        <div class="flex h-full min-h-0 min-w-0 pl-4">
          <GateTaskDetail
            :task="activeTask"
            :body="body"
            :history="history"
            :body-loading="bodyLoading"
            :history-loading="historyLoading"
            :body-error="bodyError"
            :history-error="historyError"
            @close="$emit('close')"
          />
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  </div>
</template>
