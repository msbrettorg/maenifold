<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { Search, RotateCcw, RefreshCw, Network, ArrowLeft, Loader2, Sparkles } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import GraphViewCanvas from './GraphViewCanvas.vue';
import { useForceGraph } from '../composables/useForceGraph';
import type { ForceEdge, ForceNode } from '../lib/graph-types';
import { useKnowledgeStore } from '../stores/knowledge';

const conceptFilter = ref('');
const containerRef = ref<HTMLElement | null>(null);
const store = useKnowledgeStore();
const {
  graphLoading,
  graphError,
  communityGraph,
  conceptGraph,
  conceptNodes,
  activeCommunityId,
  selectedConcept,
  conceptContextLoading,
  conceptContextError,
  relatedConcepts,
  relatedConceptsSource,
  relatedConceptsLoading,
  relatedConceptsError,
} = storeToRefs(store);

const viewLevel = computed(() => (activeCommunityId.value === null ? 'communities' : 'concepts'));
const nodeCount = computed(() => (
  viewLevel.value === 'communities'
    ? (communityGraph.value?.nodes.length ?? 0)
    : (conceptGraph.value?.nodes.length ?? 0)
));

const filteredConcepts = computed(() => {
  const query = conceptFilter.value.trim().toLowerCase();
  const base = conceptNodes.value;
  if (!query) {
    return base;
  }
  return base.filter((item) => item.name.toLowerCase().includes(query));
});

// Golden-angle HSL palette: desaturated, high-lightness, consistent with the
// near-white aesthetic of mind.html. Generated on demand from the index.
function colorForCommunityIndex(index: number): string {
  const hue = (index * 137.508) % 360;
  return `hsl(${hue.toFixed(2)}, 40%, 75%)`;
}

// sqrt scaling keeps the dynamic range tight so high-degree nodes stay readable
// without dominating the canvas. Mirrors mind.html's 1.6 → 6.5 px range.
function radiusForDegree(degree: number, maxDegree: number): number {
  if (maxDegree <= 0) return 1.6;
  const ratio = Math.max(0, Math.min(1, degree / maxDegree));
  return 1.6 + 4.9 * Math.sqrt(ratio);
}

// Map degree to luma — high-degree nodes brighter (#f0f0f0), low-degree dimmer (#a0a0a0).
function lumaForDegree(degree: number, maxDegree: number): string {
  if (maxDegree <= 0) return '#c0c0c0';
  const ratio = Math.max(0, Math.min(1, degree / maxDegree));
  // 0xa0 (160) → 0xf0 (240)
  const value = Math.round(160 + ratio * 80);
  const hex = value.toString(16).padStart(2, '0');
  return `#${hex}${hex}${hex}`;
}

function mapCommunityGraph(): { nodes: ForceNode[]; edges: ForceEdge[] } {
  const graph = communityGraph.value;
  if (!graph) {
    return { nodes: [], edges: [] };
  }

  const maxSize = graph.nodes.reduce((max, n) => Math.max(max, n.size), 0);

  const nodes = graph.nodes.map((node, index) => ({
    id: `community-${node.id}`,
    label: node.anchor,
    radius: radiusForDegree(node.size, maxSize),
    color: colorForCommunityIndex(index),
  }));

  const known = new Set(nodes.map((node) => node.id));
  const edges = graph.edges
    .filter((edge) => known.has(`community-${edge.source}`) && known.has(`community-${edge.target}`))
    .map((edge) => ({
      source: `community-${edge.source}`,
      target: `community-${edge.target}`,
      weight: edge.weight,
    }));

  return { nodes, edges };
}

function mapConceptGraph(): { nodes: ForceNode[]; edges: ForceEdge[] } {
  const graph = conceptGraph.value;
  if (!graph) {
    return { nodes: [], edges: [] };
  }

  const maxOccurrences = graph.nodes.reduce((max, n) => Math.max(max, n.occurrences), 0);

  const nodes = graph.nodes.map((node) => ({
    id: node.name,
    label: node.name,
    radius: radiusForDegree(node.occurrences, maxOccurrences),
    color: node.name === graph.anchor
      ? '#FFFFFF'
      : lumaForDegree(node.occurrences, maxOccurrences),
  }));

  const known = new Set(nodes.map((node) => node.id));
  const edges = graph.edges
    .filter((edge) => known.has(edge.source) && known.has(edge.target))
    .map((edge) => ({
      source: edge.source,
      target: edge.target,
      weight: edge.weight,
    }));

  return { nodes, edges };
}

function handleNodeClick(node: ForceNode) {
  if (viewLevel.value === 'communities') {
    const id = Number.parseInt(node.id.replace('community-', ''), 10);
    if (Number.isFinite(id)) {
      void store.focusCommunity(id);
    }
    return;
  }

  void store.openConcept(node.id);
}

const { start, stop, resetView } = useForceGraph(containerRef, undefined, handleNodeClick);

async function redrawGraph() {
  const mapped = viewLevel.value === 'communities' ? mapCommunityGraph() : mapConceptGraph();
  await nextTick();
  start(mapped.nodes, mapped.edges);
}

function activeCommunityTitle(): string {
  if (activeCommunityId.value === null || !communityGraph.value) {
    return 'Concept communities';
  }
  const match = communityGraph.value.nodes.find((node) => node.id === activeCommunityId.value);
  return match?.anchor ?? `Community ${activeCommunityId.value}`;
}

async function reloadCurrentView() {
  if (viewLevel.value === 'communities') {
    await store.loadGraphCommunities();
  } else if (activeCommunityId.value !== null) {
    await store.focusCommunity(activeCommunityId.value);
  }
}

watch(
  () => [viewLevel.value, communityGraph.value, conceptGraph.value],
  () => {
    if (graphLoading.value || graphError.value) {
      stop();
      return;
    }
    void redrawGraph();
  },
  { deep: true },
);

watch(activeCommunityId, () => {
  conceptFilter.value = '';
});

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
  <section class="flex min-h-0 flex-1 border-r border-border-subtle bg-bg-app">
    <aside class="flex min-h-0 w-80 shrink-0 flex-col border-r border-border-subtle bg-bg-sidebar">
      <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
        <Network class="size-4 text-text-secondary" aria-hidden="true" />
        <p class="truncate text-[var(--text-body-sm)] font-semibold text-text-primary">{{ activeCommunityTitle() }}</p>
        <Badge variant="secondary" class="ml-auto">{{ nodeCount }} nodes</Badge>
      </div>

      <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
        <Button
          v-if="viewLevel === 'concepts'"
          type="button"
          size="icon-sm"
          variant="ghost"
          class="text-text-secondary hover:text-text-primary"
          title="Back to communities"
          @click="store.showCommunityGraph()"
        >
          <ArrowLeft class="size-4" aria-hidden="true" />
        </Button>
        <div class="relative min-w-0 flex-1">
          <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
          <Input
            v-model="conceptFilter"
            :disabled="viewLevel !== 'concepts'"
            class="h-8 border-border-subtle bg-bg-surface pl-8 text-[var(--text-body-sm)] text-text-primary placeholder:text-text-secondary disabled:opacity-60"
            placeholder="Filter concepts"
          />
        </div>
      </div>

      <ScrollArea class="min-h-0 flex-1 px-2 py-2">
        <div class="grid gap-1">
          <template v-if="graphLoading && viewLevel === 'concepts'">
            <Skeleton class="h-8 rounded-[var(--radius-8)]" />
            <Skeleton class="h-8 rounded-[var(--radius-8)]" />
            <Skeleton class="h-8 rounded-[var(--radius-8)]" />
          </template>
          <template v-else-if="viewLevel === 'concepts'">
            <Button
              v-for="concept in filteredConcepts"
              :key="concept.name"
              type="button"
              variant="ghost"
              class="h-8 justify-start gap-2 rounded-[var(--radius-8)] px-2 text-left"
              :class="selectedConcept === concept.name ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
              @click="store.openConcept(concept.name)"
            >
              <span class="truncate text-[var(--text-body-sm)]">{{ concept.name }}</span>
              <span class="ml-auto text-[var(--text-tiny)] text-text-tertiary">{{ concept.edgeCount }}</span>
            </Button>
            <p v-if="filteredConcepts.length === 0" class="px-2 py-3 text-[var(--text-body-sm)] text-text-secondary">
              No concepts match this filter.
            </p>
          </template>
          <p v-else class="px-2 py-3 text-[var(--text-body-sm)] text-text-secondary">
            Select a community cluster in the graph to browse concepts.
          </p>
        </div>
      </ScrollArea>
    </aside>

    <div class="flex min-h-0 min-w-0 flex-1 flex-col">
      <div class="flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-2">
        <div class="inline-flex items-center gap-2 text-[var(--text-body-sm)] text-text-secondary">
          <span>{{ viewLevel === 'communities' ? 'Community map' : 'Concept map' }}</span>
          <span class="text-text-tertiary">·</span>
          <span class="text-text-tertiary">Click a node to open context</span>
        </div>
        <div class="inline-flex items-center gap-2">
          <Button
            type="button"
            size="sm"
            variant="ghost"
            class="h-8 text-text-secondary hover:text-text-primary"
            @click="resetView"
          >
            <RotateCcw class="size-4" aria-hidden="true" />
            Reset
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            class="h-8 text-text-secondary hover:text-text-primary"
            @click="reloadCurrentView"
          >
            <RefreshCw class="size-4" aria-hidden="true" />
            Reload
          </Button>
        </div>
      </div>

      <GraphViewCanvas :is-loading="graphLoading" :error="graphError ?? ''" :node-count="nodeCount" @retry="reloadCurrentView">
        <div ref="containerRef" class="relative h-full w-full" />
      </GraphViewCanvas>
    </div>

    <aside class="flex min-h-0 w-72 shrink-0 flex-col border-l border-border-subtle bg-bg-sidebar">
      <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
        <Sparkles class="size-4 text-text-secondary" aria-hidden="true" />
        <p class="text-[var(--text-body-sm)] font-semibold text-text-primary">Related concepts</p>
      </div>
      <div class="border-b border-border-subtle px-3 py-2 text-[var(--text-tiny)] text-text-tertiary">
        {{ relatedConceptsSource ? `Seed: ${relatedConceptsSource}` : 'Open a note to load nearby concepts.' }}
      </div>
      <ScrollArea class="min-h-0 flex-1 px-2 py-2">
        <div class="grid gap-2">
          <Alert v-if="relatedConceptsError" variant="destructive" class="border-border-subtle bg-bg-surface">
            <AlertTitle>Could not load related concepts</AlertTitle>
            <AlertDescription>{{ relatedConceptsError }}</AlertDescription>
          </Alert>

          <template v-else-if="relatedConceptsLoading">
            <div class="inline-flex items-center gap-2 px-2 py-1 text-[var(--text-body-sm)] text-text-secondary">
              <Loader2 class="size-4 animate-spin" aria-hidden="true" />
              Loading related concepts…
            </div>
            <Skeleton class="h-8 rounded-[var(--radius-8)]" />
            <Skeleton class="h-8 rounded-[var(--radius-8)]" />
          </template>

          <template v-else-if="relatedConcepts.length > 0">
            <Button
              v-for="concept in relatedConcepts"
              :key="concept.name"
              type="button"
              variant="ghost"
              class="h-auto justify-start rounded-[var(--radius-8)] px-2 py-2 text-left"
              @click="store.openConcept(concept.name)"
            >
              <div class="grid min-w-0 gap-1">
                <span class="truncate text-[var(--text-body-sm)] text-text-primary">{{ concept.name }}</span>
                <span class="text-[var(--text-tiny)] text-text-tertiary">
                  Similarity {{ concept.similarity.toFixed(3) }}
                </span>
              </div>
            </Button>
          </template>

          <p v-else class="px-2 py-3 text-[var(--text-body-sm)] text-text-secondary">
            No related concepts available for the selected note.
          </p>
        </div>
      </ScrollArea>
      <Alert
        v-if="conceptContextError"
        variant="destructive"
        class="m-2 border-border-subtle bg-bg-surface"
      >
        <AlertTitle>Concept navigation failed</AlertTitle>
        <AlertDescription>{{ conceptContextError }}</AlertDescription>
      </Alert>
      <div
        v-else-if="conceptContextLoading"
        class="px-3 pb-3 text-[var(--text-tiny)] text-text-secondary"
      >
        Opening concept context…
      </div>
    </aside>
  </section>
</template>
