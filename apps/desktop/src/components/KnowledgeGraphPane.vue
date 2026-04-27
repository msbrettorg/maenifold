<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { ArrowLeft, Network, RefreshCw, RotateCcw } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import GraphViewCanvas from './GraphViewCanvas.vue';
import KnowledgeConceptList from './KnowledgeConceptList.vue';
import KnowledgeRelatedConcepts from './KnowledgeRelatedConcepts.vue';
import { useForceGraph } from '../composables/useForceGraph';
import type { ForceNode } from '../lib/graph-types';
import { mapCommunityGraph, mapConceptGraph } from '../lib/knowledge-graph-mappers';
import { useKnowledgeStore } from '../stores/knowledge';

const containerRef = ref<HTMLElement | null>(null);
const store = useKnowledgeStore();
const {
  graphLoading,
  graphError,
  communityGraph,
  conceptGraph,
  activeCommunityId,
} = storeToRefs(store);

const viewLevel = computed(() => (activeCommunityId.value === null ? 'communities' : 'concepts'));
const nodeCount = computed(() => (
  viewLevel.value === 'communities'
    ? (communityGraph.value?.nodes.length ?? 0)
    : (conceptGraph.value?.nodes.length ?? 0)
));

function handleNodeClick(node: ForceNode): void {
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

async function redrawGraph(): Promise<void> {
  const mapped = viewLevel.value === 'communities'
    ? mapCommunityGraph(communityGraph.value)
    : mapConceptGraph(conceptGraph.value);
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

async function reloadCurrentView(): Promise<void> {
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
  <section class="flex h-full min-h-0 min-w-0 flex-col border-l border-border-subtle bg-bg-app" aria-label="Knowledge graph">
    <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
      <Network class="size-4 text-text-secondary" aria-hidden="true" />
      <p class="truncate text-sm font-semibold text-text-primary">{{ activeCommunityTitle() }}</p>
      <Badge variant="secondary" class="ml-auto">{{ nodeCount }} nodes</Badge>
    </div>

    <div class="flex min-h-0 flex-1">
      <aside class="flex min-h-0 w-56 shrink-0 flex-col border-r border-border-subtle bg-bg-sidebar">
        <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
          <Button
            v-if="viewLevel === 'concepts'"
            type="button"
            size="icon-sm"
            variant="ghost"
            class="text-text-secondary hover:text-text-primary"
            @click="store.showCommunityGraph()"
          >
            <ArrowLeft class="size-4" aria-hidden="true" />
          </Button>
          <span class="text-sm text-text-secondary">
            {{ viewLevel === 'communities' ? 'Communities' : 'Concepts' }}
          </span>
        </div>
        <KnowledgeConceptList />
      </aside>

      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <div class="flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-2">
          <div class="min-w-0 truncate text-sm text-text-secondary">
            {{ viewLevel === 'communities' ? 'Community map' : 'Concept map' }}
            <span class="text-text-tertiary"> · Click a node to open context</span>
          </div>
          <div class="inline-flex items-center gap-2">
            <Button type="button" size="sm" variant="ghost" class="h-8 text-text-secondary hover:text-text-primary" @click="resetView">
              <RotateCcw class="size-4" aria-hidden="true" />
              Reset
            </Button>
            <Button type="button" size="sm" variant="ghost" class="h-8 text-text-secondary hover:text-text-primary" @click="reloadCurrentView">
              <RefreshCw class="size-4" aria-hidden="true" />
              Reload
            </Button>
          </div>
        </div>

        <GraphViewCanvas :is-loading="graphLoading" :error="graphError ?? ''" :node-count="nodeCount" @retry="reloadCurrentView">
          <div ref="containerRef" class="relative h-full w-full" />
        </GraphViewCanvas>
      </div>
    </div>

    <KnowledgeRelatedConcepts />
  </section>
</template>
