<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import GraphViewCanvas from './GraphViewCanvas.vue';
import KnowledgeGraphCommunities from './KnowledgeGraphCommunities.vue';
import KnowledgeGraphToolbar from './KnowledgeGraphToolbar.vue';
import KnowledgeNodeInspector from './KnowledgeNodeInspector.vue';
import { useForceGraph } from '../composables/useForceGraph';
import type { ForceEdge, ForceNode } from '../lib/graph-types';
import { mapCommunityGraph, mapConceptGraph } from '../lib/knowledge-graph-mappers';
import { useKnowledgeStore } from '../stores/knowledge';

const containerRef = ref<HTMLElement | null>(null);
const graphFilter = ref('');
const hoveredNode = ref<ForceNode | null>(null);
const store = useKnowledgeStore();
const {
  graphLoading,
  graphError,
  communityGraph,
  conceptGraph,
  activeCommunityId,
} = storeToRefs(store);

const viewLevel = computed(() => (activeCommunityId.value === null ? 'communities' : 'concepts'));
const communityOptions = computed(() => communityGraph.value?.nodes ?? []);
const selectedCommunity = computed(() => (
  activeCommunityId.value === null ? 'all' : String(activeCommunityId.value)
));

const mappedGraph = computed(() => (
  viewLevel.value === 'communities'
    ? mapCommunityGraph(communityGraph.value)
    : mapConceptGraph(conceptGraph.value)
));

const visibleGraph = computed((): { nodes: ForceNode[]; edges: ForceEdge[] } => {
  const query = graphFilter.value.trim().toLowerCase();
  if (!query) return mappedGraph.value;

  const nodes = mappedGraph.value.nodes.filter((node) => (
    node.label.toLowerCase().includes(query) || node.id.toLowerCase().includes(query)
  ));
  const ids = new Set(nodes.map((node) => node.id));
  const edges = mappedGraph.value.edges.filter((edge) => {
    const source = typeof edge.source === 'object' ? edge.source.id : edge.source;
    const target = typeof edge.target === 'object' ? edge.target.id : edge.target;
    return ids.has(source) && ids.has(target);
  });
  return { nodes, edges };
});

const nodeCount = computed(() => visibleGraph.value.nodes.length);
const hoveredFileCount = computed(() => {
  const node = hoveredNode.value;
  if (!node) return 0;
  if (viewLevel.value === 'communities') {
    return communityGraph.value?.nodes.find((item) => `community-${item.id}` === node.id)?.size ?? 0;
  }
  return conceptGraph.value?.nodes.find((item) => item.name === node.id)?.occurrences ?? 0;
});
const title = computed(() => {
  if (activeCommunityId.value === null || !communityGraph.value) {
    return 'Concept communities';
  }
  const match = communityGraph.value.nodes.find((node) => node.id === activeCommunityId.value);
  return match?.anchor ?? `Community ${activeCommunityId.value}`;
});

function handleNodeClick(node: ForceNode): void {
  if (viewLevel.value === 'communities') {
    const id = Number.parseInt(node.id.replace('community-', ''), 10);
    if (Number.isFinite(id)) {
      void store.focusCommunity(id);
    }
    return;
  }
  void store.inspectConcept(node.id);
}

function handleNodeDoubleClick(node: ForceNode): void {
  if (viewLevel.value === 'concepts') {
    void store.openConceptInMemory(node.id);
  }
}

const { start, stop, resetView } = useForceGraph(
  containerRef,
  (node) => { hoveredNode.value = node; },
  handleNodeClick,
  handleNodeDoubleClick,
);

async function redrawGraph(): Promise<void> {
  await nextTick();
  start(visibleGraph.value.nodes, visibleGraph.value.edges);
}

async function reloadCurrentView(): Promise<void> {
  if (viewLevel.value === 'communities') {
    await store.loadGraphCommunities();
  } else if (activeCommunityId.value !== null) {
    await store.focusCommunity(activeCommunityId.value);
  }
}

function selectCommunity(value: string | number | null): void {
  if (value === null) return;
  if (value === 'all') {
    store.showCommunityGraph();
    return;
  }
  const id = Number.parseInt(String(value), 10);
  if (Number.isFinite(id)) {
    void store.focusCommunity(id);
  }
}

watch(
  () => [graphLoading.value, graphError.value, visibleGraph.value],
  () => {
    if (graphLoading.value || graphError.value) {
      stop();
      return;
    }
    void redrawGraph();
  },
  { deep: true },
);

onMounted(async () => {
  if (!communityGraph.value) {
    await store.loadGraphCommunities();
  }
  if (!graphError.value) {
    await redrawGraph();
  }
});
</script>

<template>
  <section class="flex h-full min-h-0 min-w-0 flex-1 bg-bg-app" aria-label="Knowledge graph">
    <ResizablePanelGroup
      direction="horizontal"
      auto-save-id="knowledge-graph-layout"
      class="h-full w-full"
    >
      <ResizablePanel id="graph-communities" :default-size="18" :min-size="15" :max-size="30" :order="1">
        <KnowledgeGraphCommunities />
      </ResizablePanel>
      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />
      <ResizablePanel id="graph-canvas" :default-size="62" :min-size="30" :order="2">
        <div class="flex h-full min-h-0 min-w-0 flex-col bg-bg-app">
          <KnowledgeGraphToolbar
            :title="title"
            :node-count="nodeCount"
            :filter="graphFilter"
            :selected-community="selectedCommunity"
            :communities="communityOptions"
            :is-loading="graphLoading"
            @update:filter="graphFilter = String($event)"
            @select-community="selectCommunity"
            @reset="resetView"
            @reload="reloadCurrentView"
          />
          <GraphViewCanvas :is-loading="graphLoading" :error="graphError ?? ''" :node-count="nodeCount" @retry="reloadCurrentView">
            <div ref="containerRef" class="relative h-full w-full bg-bg-app" />
            <div
              v-if="hoveredNode"
              class="pointer-events-none absolute right-4 top-4 z-[1] rounded-lg border border-border-subtle bg-bg-surface/95 px-3 py-2 shadow-lg"
            >
              <p class="max-w-64 truncate text-sm font-medium text-text-primary">{{ hoveredNode.label }}</p>
              <p class="text-xs text-text-secondary">{{ hoveredFileCount }} files</p>
            </div>
          </GraphViewCanvas>
        </div>
      </ResizablePanel>
      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />
      <ResizablePanel
        id="graph-inspector"
        :default-size="20"
        :min-size="15"
        :max-size="35"
        :collapsed-size="0"
        collapsible
        :order="3"
      >
        <KnowledgeNodeInspector />
      </ResizablePanel>
    </ResizablePanelGroup>
  </section>
</template>
