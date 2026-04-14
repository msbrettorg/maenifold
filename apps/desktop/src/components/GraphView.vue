<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue';
import GraphViewCanvas from './GraphViewCanvas.vue';
import GraphViewControls from './GraphViewControls.vue';
import GraphViewLegend from './GraphViewLegend.vue';
import { useForceGraph } from '../composables/useForceGraph';
import type {
  CommunityGraph,
  ConceptGraph,
  ForceEdge,
  ForceNode,
} from '../lib/graph-types';
import { getDaemonUrl } from '../lib/daemon-url';

const containerRef = ref<HTMLElement | null>(null);
const isLoading = ref(false);
const error = ref('');
const viewLevel = ref<'communities' | 'concepts'>('communities');
const activeCommunity = ref<{ id: number; anchor: string } | null>(null);
const nodeCount = ref(0);
const edgeCount = ref(0);

const COMMUNITY_COLORS = [
  '#4d8eff', '#4edea3', '#F59E0B', '#03a9f4',
  '#10B981', '#adc6ff', '#f44336', '#4da6ff',
  '#80cbc4', '#ffcc80', '#9e9e9e', '#ef9a9a',
];

function colorForIndex(index: number): string {
  return COMMUNITY_COLORS[index % COMMUNITY_COLORS.length];
}

function handleHover(_node: ForceNode | null) {
  // force-graph handles tooltip natively via nodeLabel
}

function handleClick(node: ForceNode) {
  if (viewLevel.value === 'communities') {
    const communityId = parseInt(node.id.replace('c-', ''), 10);
    activeCommunity.value = { id: communityId, anchor: node.label };
    viewLevel.value = 'concepts';
  }
}

const { start, stop, resetView } = useForceGraph(
  containerRef,
  handleHover,
  handleClick,
);

function navigateBack() {
  activeCommunity.value = null;
  viewLevel.value = 'communities';
}

async function reloadCurrentView() {
  if (viewLevel.value === 'communities') {
    await loadCommunities();
    return;
  }

  if (activeCommunity.value) {
    await loadConcepts(activeCommunity.value.id);
  }
}

async function loadCommunities() {
  isLoading.value = true;
  error.value = '';
  try {
    const response = await fetch(getDaemonUrl('/api/graph/communities'));
    if (!response.ok) {
      throw new Error((await response.text()) || `Failed to load communities (${response.status}).`);
    }
    const data = await response.json() as CommunityGraph;
    nodeCount.value = data.nodes.length;
    edgeCount.value = data.edges.length;

    const allNodes: ForceNode[] = data.nodes.map((n, i) => ({
      id: `c-${n.id}`,
      label: n.anchor,
      radius: Math.max(4, Math.log2(n.size + 1) * 4),
      color: colorForIndex(i),
    }));

    const idSet = new Set(allNodes.map((n) => n.id));
    const forceEdges: ForceEdge[] = data.edges
      .filter((e) => idSet.has(`c-${e.source}`) && idSet.has(`c-${e.target}`))
      .map((e) => ({
        source: `c-${e.source}`,
        target: `c-${e.target}`,
        weight: e.weight,
      }));

    // Only show connected communities — no orphans on the community view
    const connectedIds = new Set<string>();
    for (const e of forceEdges) {
      connectedIds.add(String(e.source));
      connectedIds.add(String(e.target));
    }
    const forceNodes = allNodes.filter((n) => connectedIds.has(n.id));

    nodeCount.value = forceNodes.length;
    edgeCount.value = forceEdges.length;

    await nextTick();
    start(forceNodes, forceEdges);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    isLoading.value = false;
  }
}

async function loadConcepts(communityId: number) {
  isLoading.value = true;
  error.value = '';
  try {
    const response = await fetch(getDaemonUrl(`/api/graph/communities/${communityId}/concepts`));
    if (!response.ok) {
      throw new Error((await response.text()) || `Failed to load concepts (${response.status}).`);
    }
    const data = await response.json() as ConceptGraph;
    nodeCount.value = data.nodes.length;
    edgeCount.value = data.edges.length;

    const parentColor = colorForIndex(communityId % COMMUNITY_COLORS.length);

    const forceNodes: ForceNode[] = data.nodes.map((n) => ({
      id: n.name,
      label: n.name,
      radius: Math.max(3, Math.log2(n.occurrences + 1) * 2.5),
      color: n.name === data.anchor ? parentColor : dimColor(parentColor, 0.7),
    }));

    const nameSet = new Set(forceNodes.map((n) => n.id));
    const forceEdges: ForceEdge[] = data.edges
      .filter((e) => nameSet.has(e.source) && nameSet.has(e.target))
      .map((e) => ({
        source: e.source,
        target: e.target,
        weight: e.weight,
      }));

    await nextTick();
    start(forceNodes, forceEdges);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    isLoading.value = false;
  }
}

function dimColor(hex: string, factor: number): string {
  const r = Math.round(parseInt(hex.slice(1, 3), 16) * factor);
  const g = Math.round(parseInt(hex.slice(3, 5), 16) * factor);
  const b = Math.round(parseInt(hex.slice(5, 7), 16) * factor);
  return `rgb(${r}, ${g}, ${b})`;
}

watch(viewLevel, async (level) => {
  stop();
  if (level === 'communities') {
    await loadCommunities();
  } else if (activeCommunity.value) {
    await loadConcepts(activeCommunity.value.id);
  }
});

onMounted(async () => {
  await nextTick();
  await loadCommunities();
});
</script>

<template>
  <section class="graph-container" role="application" aria-label="Knowledge graph">
    <GraphViewControls
      :view-level="viewLevel"
      :active-community-anchor="activeCommunity?.anchor ?? null"
      :node-count="nodeCount"
      :edge-count="edgeCount"
      @navigate-back="navigateBack"
      @reset="resetView"
      @reload="reloadCurrentView"
    />
    <GraphViewLegend :view-level="viewLevel" :active-community-anchor="activeCommunity?.anchor ?? null" />
    <GraphViewCanvas :is-loading="isLoading" :error="error" :node-count="nodeCount" @retry="reloadCurrentView">
      <div ref="containerRef" class="graph-canvas" />
    </GraphViewCanvas>
  </section>
</template>

<style scoped>
.graph-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
}
.graph-canvas {
  width: 100%;
  height: 100%;
  position: relative;
  background: #0e0e0e;
}
</style>
