<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue';
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
  <section class="graph-container">
    <!-- Toolbar -->
    <div class="graph-toolbar">
      <div class="graph-breadcrumb">
        <button
          v-if="viewLevel === 'concepts'"
          class="breadcrumb-link"
          @click="navigateBack"
        >
          ← Communities
        </button>
        <span v-if="viewLevel === 'concepts'" class="breadcrumb-sep">/</span>
        <span class="breadcrumb-current">
          {{ viewLevel === 'communities' ? 'Knowledge Graph' : activeCommunity?.anchor }}
        </span>
      </div>

      <div v-if="nodeCount > 0" class="graph-stats">
        <span class="stat">{{ nodeCount }} nodes</span>
        <span class="stat-sep">·</span>
        <span class="stat">{{ edgeCount }} edges</span>
      </div>

      <div class="toolbar-actions">
        <button class="graph-action-btn" @click="resetView" title="Reset view">↺</button>
        <button
          class="graph-action-btn"
          @click="viewLevel === 'communities' ? loadCommunities() : activeCommunity && loadConcepts(activeCommunity.id)"
          title="Reload data"
        >⟳</button>
      </div>
    </div>

    <!-- Graph container -->
    <div class="graph-canvas-wrap">
      <!-- Empty state overlay -->
      <div v-if="error && !isLoading" class="graph-empty-state">
        <div class="empty-icon">◎</div>
        <p class="empty-title">No graph connection</p>
        <p class="empty-hint">Launch maenifold to explore the knowledge graph.</p>
        <button class="empty-retry" @click="loadCommunities">Retry</button>
      </div>

      <!-- Loading state -->
      <div v-else-if="isLoading" class="graph-loading">
        <div class="loading-ring" />
        <p class="loading-text">Loading graph…</p>
      </div>

      <div ref="containerRef" class="graph-canvas" />
    </div>
  </section>
</template>

<style scoped>
.graph-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
}

.graph-toolbar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-panel);
  flex-shrink: 0;
}

.graph-breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: var(--text-caption);
  color: var(--text-secondary);
  flex: 1;
}

.breadcrumb-link {
  background: none;
  border: none;
  color: var(--border-focus);
  cursor: pointer;
  padding: 0;
  font-size: var(--text-caption);
}

.breadcrumb-link:hover {
  text-decoration: underline;
}

.breadcrumb-sep {
  color: var(--text-muted);
}

.breadcrumb-current {
  color: var(--text-primary);
  font-weight: var(--weight-display);
  font-family: var(--font-display);
  letter-spacing: 0.02em;
}

.graph-stats {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-tiny);
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

.stat-sep {
  opacity: 0.4;
}

.toolbar-actions {
  display: flex;
  gap: 0.25rem;
}

.graph-action-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-default);
  background: var(--bg-surface);
  color: var(--text-muted);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--text-body-sm);
  line-height: 1;
  transition: var(--transition-fast);
}

.graph-action-btn:hover {
  background: var(--bg-surface);
  color: var(--text-primary);
  border-color: var(--text-muted);
}

.graph-canvas-wrap {
  flex: 1;
  position: relative;
  overflow: hidden;
  background:
    radial-gradient(ellipse at 30% 50%, rgba(77, 142, 255, 0.03) 0%, transparent 60%),
    radial-gradient(ellipse at 70% 40%, rgba(78, 222, 163, 0.02) 0%, transparent 50%),
    var(--bg-main);
}

.graph-canvas {
  width: 100%;
  height: 100%;
  position: relative;
  background: var(--bg-main);
}

/* Empty state */
.graph-empty-state {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 2;
  background: radial-gradient(ellipse at center, rgba(77, 142, 255, 0.03) 0%, transparent 70%);
}

.empty-icon {
  font-size: var(--text-display);
  color: var(--text-muted);
  opacity: 0.3;
  margin-bottom: 1rem;
}

.empty-title {
  font-family: var(--font-display);
  font-size: var(--text-lg);
  color: var(--text-secondary);
  margin-bottom: 0.25rem;
}

.empty-hint {
  font-size: var(--text-caption);
  color: var(--text-muted);
  margin-bottom: 1.25rem;
}

.empty-retry {
  padding: 0.4rem 1.2rem;
  border: 1px solid var(--border-default);
  background: var(--bg-surface);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: var(--text-caption);
  transition: var(--transition-fast);
}

.empty-retry:hover {
  background: var(--bg-surface);
  color: var(--text-primary);
  border-color: var(--border-focus);
}

/* Loading state */
.graph-loading {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 2;
}

.loading-ring {
  width: 32px;
  height: 32px;
  border: 2px solid var(--border-default);
  border-top-color: var(--border-focus);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 0.75rem;
}

.loading-text {
  font-size: var(--text-caption);
  color: var(--text-muted);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
