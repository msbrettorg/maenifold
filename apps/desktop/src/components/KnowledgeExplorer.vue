<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { Search, X, BookOpenText, Network } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import KnowledgeContent from './KnowledgeContent.vue';
import KnowledgeGraphPanel from './KnowledgeGraphPanel.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import KnowledgeSidebar from './KnowledgeSidebar.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useKnowledgeStore } from '../stores/knowledge';
import { useProvenanceStore } from '../stores/provenance';

const store = useKnowledgeStore();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();
const KNOWLEDGE_SPLITTER_RATIO_KEY = 'knowledge-splitter-ratio';
const DEFAULT_SIDEBAR_RATIO = 0.3;
const MIN_SIDEBAR_WIDTH = 200;
const MIN_CONTENT_WIDTH = 400;
const MIN_TOTAL_WIDTH = MIN_SIDEBAR_WIDTH + MIN_CONTENT_WIDTH;
const FALLBACK_CONTAINER_WIDTH = 1200;

const query = ref(store.searchQuery);
const mode = ref<'notes' | 'graph'>('notes');
const notesContainerEl = ref<HTMLElement | null>(null);
const notesContainerWidth = ref(FALLBACK_CONTAINER_WIDTH);
const storedSidebarRatio = ref(readStoredRatio());
const currentKnowledgeLineageId = computed(() => store.activeUri);
const currentKnowledgeTitle = computed(() => store.activeNote?.title);
const activeNoteLineageLinks = computed(() =>
  provenance.buildLineageLinks({
    gate: store.activeNote?.lineageGateTaskId,
    pipeline: store.activeNote?.lineagePipelineRunId,
    thinking: store.activeNote?.lineageThinkingSessionId,
    knowledge: store.activeUri,
    parentWorkflowId: store.activeNote?.parentWorkflowId,
  }, {
    knowledge: currentKnowledgeTitle.value,
  }));
const provenanceLinks = computed(() =>
  activeNoteLineageLinks.value
    .filter((link) => link.kind !== 'knowledge' && link.id !== currentKnowledgeLineageId.value));

let resizeObserver: ResizeObserver | null = null;

const effectiveContainerWidth = computed(() =>
  Math.max(notesContainerWidth.value, MIN_TOTAL_WIDTH)
);
const minSidebarSize = computed(() => (MIN_SIDEBAR_WIDTH / effectiveContainerWidth.value) * 100);
const minContentSize = computed(() => (MIN_CONTENT_WIDTH / effectiveContainerWidth.value) * 100);
const maxSidebarSize = computed(() => 100 - minContentSize.value);
const defaultSidebarSize = computed(() =>
  clamp(storedSidebarRatio.value * 100, minSidebarSize.value, maxSidebarSize.value)
);
const defaultContentSize = computed(() => 100 - defaultSidebarSize.value);

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function readStoredRatio() {
  if (typeof window === 'undefined') {
    return DEFAULT_SIDEBAR_RATIO;
  }

  const raw = window.localStorage.getItem(KNOWLEDGE_SPLITTER_RATIO_KEY);
  if (raw === null) {
    return DEFAULT_SIDEBAR_RATIO;
  }

  try {
    const parsed = JSON.parse(raw) as number;
    return Number.isFinite(parsed) ? clamp(parsed, 0, 1) : DEFAULT_SIDEBAR_RATIO;
  } catch {
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? clamp(parsed, 0, 1) : DEFAULT_SIDEBAR_RATIO;
  }
}

function writeStoredRatio(value: number) {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(KNOWLEDGE_SPLITTER_RATIO_KEY, JSON.stringify(clamp(value, 0, 1)));
}

function updateContainerWidth() {
  const nextWidth = notesContainerEl.value?.clientWidth ?? 0;
  if (nextWidth > 0) {
    notesContainerWidth.value = nextWidth;
  }
}

function handleNotesLayout(layout: number[]) {
  const [sidebarSize] = layout;
  if (typeof sidebarSize !== 'number' || Number.isNaN(sidebarSize)) {
    return;
  }

  const nextRatio = clamp(sidebarSize / 100, 0, 1);
  storedSidebarRatio.value = nextRatio;
  writeStoredRatio(nextRatio);
}

async function handleSearch() {
  await store.search(query.value);
}

async function clearSearch() {
  query.value = '';
  await store.search('');
}

onMounted(async () => {
  updateContainerWidth();
  if (typeof ResizeObserver !== 'undefined' && notesContainerEl.value) {
    resizeObserver = new ResizeObserver(() => {
      updateContainerWidth();
    });
    resizeObserver.observe(notesContainerEl.value);
  }

  await store.loadRootFolders();
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <section class="flex h-full min-h-0 flex-1 flex-col bg-bg-app" aria-label="Knowledge explorer">
    <div class="px-4 pt-3">
      <ProvenanceBreadcrumbs
        current-kind="knowledge"
        :current-id="currentKnowledgeLineageId"
        :current-label="store.activeNote?.title"
        :links="provenanceLinks"
        @navigate="navigateTo"
      />
    </div>
    <header class="flex flex-wrap items-center gap-2 border-b border-border-subtle px-4 py-3">
      <Tabs
        :model-value="mode"
        @update:model-value="mode = ($event as 'notes' | 'graph')"
      >
        <TabsList variant="line" class="bg-transparent p-0">
          <TabsTrigger value="notes" class="h-8 rounded-md px-2 py-1">
            <BookOpenText class="size-4" aria-hidden="true" />
            Notes
          </TabsTrigger>
          <TabsTrigger value="graph" class="h-8 rounded-md px-2 py-1">
            <Network class="size-4" aria-hidden="true" />
            Graph
          </TabsTrigger>
        </TabsList>
      </Tabs>

      <div class="relative min-w-[16rem] flex-1">
        <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
        <Input
          v-model="query"
          class="h-9 border-border-subtle bg-bg-surface pl-8 text-text-primary placeholder:text-text-secondary"
          placeholder="Search notes..."
          @keydown.enter.prevent="handleSearch"
        />
      </div>
      <Button type="button" size="sm" class="h-9" @click="handleSearch">Search</Button>
      <Button
        v-if="query.trim().length > 0"
        type="button"
        variant="ghost"
        size="sm"
        class="h-9 text-text-secondary hover:text-text-primary"
        @click="clearSearch"
      >
        <X class="size-4" aria-hidden="true" />
        Clear
      </Button>
    </header>

    <div v-if="mode === 'notes'" ref="notesContainerEl" class="flex min-h-0 flex-1">
      <ResizablePanelGroup
        direction="horizontal"
        class="h-full w-full"
        @layout="handleNotesLayout"
      >
        <ResizablePanel
          id="knowledge-sidebar"
          :default-size="defaultSidebarSize"
          :min-size="minSidebarSize"
          :max-size="maxSidebarSize"
          :order="1"
        >
          <div class="flex h-full min-h-0 min-w-0">
            <KnowledgeSidebar />
          </div>
        </ResizablePanel>
        <ResizableHandle
          class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
        />
        <ResizablePanel
          id="knowledge-content"
          :default-size="defaultContentSize"
          :min-size="minContentSize"
          :order="2"
        >
          <div class="flex h-full min-h-0 min-w-0">
            <KnowledgeContent />
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>

    <div v-else class="flex min-h-0 flex-1">
      <KnowledgeGraphPanel />
      <div class="min-h-0 w-[32rem] shrink-0 border-l border-border-subtle">
        <KnowledgeContent />
      </div>
    </div>
  </section>
</template>
