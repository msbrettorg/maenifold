<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import {
  BookOpen,
  Brain,
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  FolderOpen,
  Network,
  RefreshCw,
  Search,
} from '@lucide/vue';
import {
  useKnowledgeStore,
  type ContextRelation,
  type MemorySearchResult,
  type MemoryTreeNode,
} from '../stores/knowledge';

interface GraphNode {
  id: string;
  label: string;
  x: number;
  y: number;
  relation?: ContextRelation;
  focus?: boolean;
}

interface MarkdownSegment {
  text: string;
  concept?: string;
}

interface MarkdownLine {
  key: string;
  kind: 'blank' | 'code' | 'h1' | 'h2' | 'h3' | 'list' | 'quote' | 'p';
  segments: MarkdownSegment[];
}

interface TreeRow {
  node: MemoryTreeNode;
  depth: number;
  expanded: boolean;
  selected: boolean;
}

const knowledge = useKnowledgeStore();
const {
  query,
  lastSearchQuery,
  search,
  searchResults,
  selectedConcept,
  stats,
  selectedMemory,
  selectedSearchResult,
  memoryTree,
  expandedTreePaths,
  loading,
  graphLoading,
  viewerLoading,
  treeLoading,
  error,
  visibleRelations,
  relationTotal,
  graphOverflow,
} = storeToRefs(knowledge);

const workspaceView = ref<'viewer' | 'graph'>('viewer');
const graphCanvas = ref<HTMLElement | null>(null);
const graphStageSize = ref<number | null>(null);

const graphStageStyle = computed<Record<string, string> | undefined>(() => {
  if (!graphStageSize.value) return undefined;
  const size = `${graphStageSize.value}px`;
  return {
    width: size,
    height: size,
  };
});

const graphNodes = computed<GraphNode[]>(() => {
  const relations = visibleRelations.value;
  const count = relations.length || 1;
  const nodes: GraphNode[] = [
    {
      id: selectedConcept.value,
      label: selectedConcept.value,
      x: 50,
      y: 50,
      focus: true,
    },
  ];

  relations.forEach((relation, index) => {
    const angle = -90 + (index * 360) / count;
    const radius = index % 2 === 0 ? 33 : 41;
    const radians = (angle * Math.PI) / 180;
    const name = relationName(relation);
    nodes.push({
      id: name,
      label: name,
      relation,
      x: 50 + Math.cos(radians) * radius,
      y: 50 + Math.sin(radians) * radius,
    });
  });

  return nodes;
});

const graphEdges = computed(() => graphNodes.value.slice(1).map((node) => node));

const treeRows = computed<TreeRow[]>(() => {
  const rows: TreeRow[] = [];
  const selectedUri = selectedMemory.value?.uri ?? selectedSearchResult.value?.path ?? '';

  function visit(nodes: MemoryTreeNode[], depth: number): void {
    nodes.forEach((node) => {
      const expanded = node.kind === 'folder' && expandedTreePaths.value.includes(node.path);
      rows.push({
        node,
        depth,
        expanded,
        selected: node.kind === 'file' && node.uri === selectedUri,
      });
      if (node.kind === 'folder' && expanded) {
        visit(node.children, depth + 1);
      }
    });
  }

  visit(memoryTree.value.children, 0);
  return rows;
});

const markdownLines = computed<MarkdownLine[]>(() => {
  const content = selectedMemory.value?.content ?? '';
  if (!content) return [];
  return content.split(/\r?\n/).map((line, index) => toMarkdownLine(line, index));
});

const searchResultCount = computed(() => search.value?.totalCount ?? searchResults.value.length);
const hasSearchState = computed(() => search.value !== null);

function formatNumber(value: number | null | undefined): string {
  if (typeof value !== 'number' || !Number.isFinite(value)) return '-';
  return new Intl.NumberFormat().format(value);
}

function formatScore(value: number | undefined | null): string {
  if (typeof value !== 'number' || !Number.isFinite(value)) return '-';
  return value.toFixed(value >= 10 ? 0 : 3);
}

function formatEdgeCount(value: number | undefined | null): string {
  if (typeof value !== 'number' || !Number.isFinite(value)) return '- edges';
  const count = Math.max(0, Math.trunc(value));
  return `${formatNumber(count)} ${count === 1 ? 'edge' : 'edges'}`;
}

function relationName(relation: ContextRelation): string {
  return relation.name ?? 'unknown-concept';
}

function treePath(node: MemoryTreeNode): string {
  return node.kind === 'file' ? node.uri ?? node.path : node.path || 'memory://';
}

function relationFiles(relation: ContextRelation): string {
  const count = relation.sourceFileCount ?? relation.files?.length ?? 0;
  return `${formatNumber(count)} files`;
}

function submitSearch(): void {
  void knowledge.runSearch(query.value, workspaceView.value === 'viewer');
}

function refreshAll(): void {
  void knowledge.refreshAll();
}

function openMemory(result: MemorySearchResult): void {
  void knowledge.readMemory(result);
  workspaceView.value = 'viewer';
}

function openGraphNode(node: GraphNode): void {
  if (node.focus) return;
  if (node.relation) void knowledge.openRelation(node.relation);
  workspaceView.value = 'viewer';
}

function openRelation(relation: ContextRelation): void {
  void knowledge.openRelation(relation);
  workspaceView.value = 'viewer';
}

async function openConcept(concept: string): Promise<void> {
  const opened = await knowledge.openConcept(concept);
  workspaceView.value = opened ? 'viewer' : 'graph';
}

function openTreeRow(row: TreeRow): void {
  if (row.node.kind === 'folder') {
    void knowledge.toggleTreeFolder(row.node.path);
    return;
  }

  if (!row.node.uri) return;
  openMemory({
    title: row.node.name,
    path: row.node.uri,
  });
}

function parseSegments(text: string): MarkdownSegment[] {
  const segments: MarkdownSegment[] = [];
  const pattern = /\[\[([^\]]+)\]\]/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > cursor) {
      segments.push({ text: text.slice(cursor, match.index) });
    }
    const concept = match[1].trim();
    segments.push({ text: concept, concept });
    cursor = match.index + match[0].length;
  }

  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor) });
  }

  return segments.length > 0 ? segments : [{ text }];
}

function toMarkdownLine(rawLine: string, index: number): MarkdownLine {
  const trimmed = rawLine.trim();
  if (!trimmed) return { key: `${index}-blank`, kind: 'blank', segments: [] };
  if (trimmed.startsWith('```')) return { key: `${index}-code`, kind: 'code', segments: [{ text: trimmed }] };
  if (trimmed.startsWith('### ')) {
    return { key: `${index}-h3`, kind: 'h3', segments: parseSegments(trimmed.slice(4)) };
  }
  if (trimmed.startsWith('## ')) {
    return { key: `${index}-h2`, kind: 'h2', segments: parseSegments(trimmed.slice(3)) };
  }
  if (trimmed.startsWith('# ')) {
    return { key: `${index}-h1`, kind: 'h1', segments: parseSegments(trimmed.slice(2)) };
  }
  if (/^[-*]\s+/.test(trimmed)) {
    return { key: `${index}-list`, kind: 'list', segments: parseSegments(trimmed.replace(/^[-*]\s+/, '')) };
  }
  if (trimmed.startsWith('>')) {
    return { key: `${index}-quote`, kind: 'quote', segments: parseSegments(trimmed.replace(/^>\s?/, '')) };
  }
  return { key: `${index}-p`, kind: 'p', segments: parseSegments(rawLine) };
}

function syncGraphStageSize(element: HTMLElement | null): void {
  if (!element) {
    graphStageSize.value = null;
    return;
  }

  const { width, height } = element.getBoundingClientRect();
  const nextSize = Math.floor(Math.min(width, height));
  graphStageSize.value = nextSize > 0 ? nextSize : null;
}

watch(graphCanvas, (element, _previous, onCleanup) => {
  syncGraphStageSize(element);

  if (!element || typeof ResizeObserver === 'undefined') {
    return;
  }

  const observer = new ResizeObserver(() => {
    syncGraphStageSize(element);
  });
  observer.observe(element);
  onCleanup(() => observer.disconnect());
});

onMounted(() => {
  void knowledge.bootstrap();
});
</script>

<template>
  <div class="knowledge-browser">
    <header class="kb-header">
      <div class="kb-title">
        <Brain :size="24" :stroke-width="1.5" />
        <div>
          <h1>Knowledge</h1>
          <p>memory:// graph://</p>
        </div>
      </div>

      <form class="kb-search" @submit.prevent="submitSearch">
        <Search :size="18" :stroke-width="2" />
        <input
          v-model="query"
          type="search"
          spellcheck="false"
          autocomplete="off"
          aria-label="Search memory"
        />
      </form>

      <button type="button" class="icon-button" :disabled="loading" aria-label="Refresh knowledge" @click="refreshAll">
        <RefreshCw :size="17" :stroke-width="2" />
      </button>

      <div class="kb-stats">
        <span>{{ formatNumber(stats?.node_count) }} concepts</span>
        <span>{{ formatNumber(stats?.edge_count) }} links</span>
      </div>
    </header>

    <p v-if="error" class="kb-error" role="status">{{ error }}</p>

    <main class="kb-layout">
      <aside class="tree-pane">
        <div class="pane-header">
          <span>Files</span>
          <strong>memory://</strong>
        </div>

        <div class="tree-list" :class="{ loading: treeLoading }">
          <button
            v-for="row in treeRows"
            :key="row.node.id"
            type="button"
            class="tree-row"
            :class="{ folder: row.node.kind === 'folder', file: row.node.kind === 'file', selected: row.selected }"
            :style="{ paddingLeft: `${0.35 + row.depth * 0.9}rem` }"
            :title="treePath(row.node)"
            @click="openTreeRow(row)"
          >
            <ChevronDown
              v-if="row.node.kind === 'folder' && row.expanded"
              class="tree-chevron"
              :size="14"
              :stroke-width="2"
            />
            <ChevronRight
              v-else-if="row.node.kind === 'folder'"
              class="tree-chevron"
              :size="14"
              :stroke-width="2"
            />
            <span v-else class="tree-chevron" aria-hidden="true"></span>

            <FolderOpen
              v-if="row.node.kind === 'folder' && row.expanded"
              class="tree-icon"
              :size="15"
              :stroke-width="1.8"
            />
            <Folder
              v-else-if="row.node.kind === 'folder'"
              class="tree-icon"
              :size="15"
              :stroke-width="1.8"
            />
            <FileText v-else class="tree-icon" :size="14" :stroke-width="1.8" />

            <span class="tree-label">{{ row.node.name }}</span>
          </button>
          <div v-if="treeRows.length === 0" class="tree-empty">
            {{ treeLoading ? 'Loading memory tree.' : 'No memory files.' }}
          </div>
        </div>
      </aside>

      <section class="workspace-pane">
        <div class="workspace-header">
          <div class="workspace-tabs" role="tablist" aria-label="Knowledge workspace">
            <button
              type="button"
              role="tab"
              class="workspace-tab"
              :class="{ active: workspaceView === 'viewer' }"
              :aria-selected="workspaceView === 'viewer'"
              @click="workspaceView = 'viewer'"
            >
              <BookOpen :size="15" :stroke-width="1.9" />
              <span>Viewer</span>
            </button>
            <button
              type="button"
              role="tab"
              class="workspace-tab"
              :class="{ active: workspaceView === 'graph' }"
              :aria-selected="workspaceView === 'graph'"
              @click="workspaceView = 'graph'"
            >
              <Network :size="15" :stroke-width="1.9" />
              <span>Graph</span>
            </button>
          </div>
          <div class="workspace-meta">
            <span>{{ selectedMemory?.title ?? selectedConcept }}</span>
            <code>{{ workspaceView === 'graph' ? selectedConcept : selectedMemory?.uri ?? 'memory://' }}</code>
          </div>
        </div>

        <section v-if="hasSearchState" class="search-results-panel">
          <div class="pane-header">
            <span>Search</span>
            <strong>{{ formatNumber(searchResultCount) }} results</strong>
          </div>

          <div class="search-results">
            <button
              v-for="result in searchResults"
              :key="result.path ?? result.title ?? 'search-result'"
              type="button"
              class="search-result-row"
              :class="{ selected: selectedSearchResult?.path === result.path }"
              @click="openMemory(result)"
            >
              <div class="search-result-main">
                <span class="search-result-title">{{ result.title ?? result.path ?? 'Untitled memory' }}</span>
                <code>{{ result.path ?? 'memory://' }}</code>
              </div>
              <small class="search-result-meta">
                <span v-if="typeof result.fusedScore === 'number'">{{ formatScore(result.fusedScore) }} ranked</span>
                <span v-if="result.snippet">{{ result.snippet }}</span>
              </small>
            </button>

            <div v-if="searchResults.length === 0" class="search-empty">
              No matches for "{{ lastSearchQuery }}"
            </div>
          </div>
        </section>

        <article v-if="workspaceView === 'viewer'" class="viewer-view">
          <div class="viewer-header">
            <span>Viewer</span>
            <h2>{{ selectedMemory?.title ?? 'No file selected' }}</h2>
            <code>{{ selectedMemory?.uri ?? 'memory://' }}</code>
          </div>

          <div class="markdown-viewer" :class="{ loading: viewerLoading }">
            <template v-if="markdownLines.length > 0">
              <div
                v-for="line in markdownLines"
                :key="line.key"
                class="md-line"
                :class="line.kind"
              >
                <span v-if="line.kind === 'list'" class="bullet">-</span>
                <template v-for="(segment, index) in line.segments" :key="`${line.key}-${index}`">
                  <button
                    v-if="segment.concept"
                    type="button"
                    class="wikilink"
                    @click="openConcept(segment.concept)"
                  >
                    {{ segment.text }}
                  </button>
                  <span v-else>{{ segment.text }}</span>
                </template>
              </div>
            </template>
            <div v-else class="empty-viewer">
              Select a memory file.
            </div>
          </div>
        </article>

        <section v-else class="graph-view">
          <div class="pane-header">
            <span>Graph</span>
            <strong>{{ formatNumber(relationTotal) }} neighbors</strong>
          </div>

          <div ref="graphCanvas" class="graph-canvas" :class="{ loading: graphLoading }">
            <div class="graph-stage" :style="graphStageStyle">
              <svg class="graph-edges" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet" aria-hidden="true">
                <line
                  v-for="node in graphEdges"
                  :key="`${selectedConcept}-${node.id}`"
                  x1="50"
                  y1="50"
                  :x2="node.x"
                  :y2="node.y"
                />
              </svg>

              <button
                v-for="node in graphNodes"
                :key="node.id"
                type="button"
                class="graph-node"
                :class="{ focus: node.focus }"
                :style="{ left: `${node.x}%`, top: `${node.y}%` }"
                @click="openGraphNode(node)"
              >
                <Network v-if="node.focus" :size="17" :stroke-width="1.8" />
                <span>{{ node.label }}</span>
              </button>
            </div>
          </div>

          <div class="graph-meta">
            <span>{{ selectedConcept }}</span>
            <span>{{ formatNumber(graphOverflow) }} hidden</span>
          </div>

          <div class="relation-list">
            <button
              v-for="relation in visibleRelations"
              :key="relationName(relation)"
              type="button"
              class="relation-row"
              @click="openRelation(relation)"
            >
              <span>{{ relationName(relation) }}</span>
              <small>
                {{ formatEdgeCount(relation.coOccurrenceCount) }} /
                {{ formatScore(relation.weightedScore) }} weighted /
                {{ relationFiles(relation) }}
              </small>
            </button>
          </div>
        </section>
      </section>
    </main>
  </div>
</template>

<style scoped>
.knowledge-browser {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--text-primary);
}

.kb-header {
  flex-shrink: 0;
  display: grid;
  grid-template-columns: minmax(190px, 0.8fr) minmax(280px, 1.3fr) auto minmax(180px, 0.6fr);
  gap: 0.75rem;
  align-items: center;
  padding: 0.8rem 1rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 88%, var(--bg-main));
}

.kb-title {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.7rem;
}

.kb-title svg {
  color: var(--text-accent);
}

.kb-title h1 {
  margin: 0;
  font-size: 1rem;
  line-height: 1.15;
}

.kb-title p,
.viewer-header code,
.workspace-meta code,
.relation-row small,
.graph-meta {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.kb-title p {
  margin: 0.1rem 0 0;
  color: var(--text-muted);
  font-size: 0.72rem;
}

.kb-search {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 0.55rem;
  height: 38px;
  padding: 0 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  color: var(--text-muted);
}

.kb-search input {
  min-width: 0;
  border: none;
  outline: none;
  color: var(--text-primary);
  background: transparent;
  font: inherit;
}

.icon-button {
  width: 38px;
  height: 38px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  color: var(--text-primary);
  cursor: pointer;
}

.icon-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.kb-stats {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  min-width: 0;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.kb-error {
  margin: 0;
  padding: 0.55rem 1rem;
  border-bottom: 1px solid color-mix(in srgb, var(--bg-danger) 42%, var(--border-color));
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
}

.kb-layout {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(260px, 330px) minmax(0, 1fr);
  gap: 0.75rem;
  padding: 0.75rem;
  overflow: hidden;
}

.tree-pane,
.workspace-pane {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  overflow: hidden;
}

.pane-header,
.viewer-header,
.workspace-header {
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 68%, transparent);
}

.pane-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.65rem 0.75rem;
  color: var(--text-secondary);
  font-size: 0.76rem;
  font-weight: 700;
  text-transform: uppercase;
}

.pane-header strong {
  color: var(--text-primary);
  font-size: 0.75rem;
  text-transform: none;
}

.tree-list,
.relation-list,
.markdown-viewer,
.search-results {
  overflow-y: auto;
}

.tree-list {
  flex: 1;
  min-height: 0;
  padding: 0.35rem;
}

.tree-list.loading {
  opacity: 0.72;
}

.tree-row,
.relation-row {
  width: 100%;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
}

.tree-row {
  display: grid;
  grid-template-columns: 16px 17px minmax(0, 1fr);
  column-gap: 0.28rem;
  align-items: center;
  min-height: 30px;
  padding-top: 0.22rem;
  padding-right: 0.45rem;
  padding-bottom: 0.22rem;
  font: inherit;
  font-size: 0.8rem;
}

.tree-row svg {
  color: var(--text-muted);
}

.tree-row.folder {
  font-weight: 650;
}

.tree-row.file {
  color: var(--text-secondary);
}

.tree-row.selected {
  color: var(--text-primary);
}

.tree-chevron,
.tree-icon {
  flex-shrink: 0;
}

.tree-chevron {
  width: 16px;
  height: 16px;
}

.tree-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-empty {
  padding: 1rem 0.7rem;
  color: var(--text-muted);
  font-size: 0.82rem;
}

.tree-row:hover,
.tree-row.selected,
.relation-row:hover,
.wikilink:hover {
  border-color: color-mix(in srgb, var(--text-accent) 42%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 9%, var(--bg-main));
}

.workspace-pane {
  min-width: 0;
}

.workspace-header {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.8rem;
  align-items: center;
  padding: 0.55rem 0.75rem;
}

.workspace-tabs {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
}

.workspace-tab {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  min-height: 32px;
  padding: 0.32rem 0.62rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.8rem;
  font-weight: 650;
  cursor: pointer;
}

.workspace-tab:hover,
.workspace-tab.active {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 34%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
}

.workspace-meta {
  min-width: 0;
  display: grid;
  justify-items: end;
  gap: 0.06rem;
}

.workspace-meta span,
.workspace-meta code {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-meta span {
  color: var(--text-primary);
  font-size: 0.82rem;
  font-weight: 650;
}

.workspace-meta code {
  color: var(--text-muted);
  font-size: 0.68rem;
}

.viewer-view,
.graph-view,
.search-results-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.search-results-panel {
  flex: 0 0 180px;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
}

.search-results {
  min-height: 0;
  padding: 0.45rem;
}

.search-result-row {
  width: 100%;
  display: grid;
  gap: 0.22rem;
  margin: 0 0 0.35rem;
  padding: 0.55rem 0.6rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
}

.search-result-row:last-child {
  margin-bottom: 0;
}

.search-result-row:hover,
.search-result-row.selected {
  border-color: color-mix(in srgb, var(--text-accent) 42%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 9%, var(--bg-main));
}

.search-result-main {
  display: grid;
  gap: 0.12rem;
}

.search-result-title {
  font-size: 0.84rem;
  font-weight: 650;
  overflow-wrap: anywhere;
}

.search-result-main code,
.search-result-meta {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.search-result-main code {
  color: var(--text-muted);
  font-size: 0.66rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-result-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  color: var(--text-muted);
  font-size: 0.68rem;
}

.search-empty {
  padding: 0.7rem 0.6rem;
  color: var(--text-muted);
  font-size: 0.8rem;
}

.graph-canvas {
  position: relative;
  flex: 1;
  min-height: 330px;
  margin: 0.7rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background:
    radial-gradient(circle at 50% 50%, color-mix(in srgb, var(--text-accent) 10%, transparent), transparent 42%),
    color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  overflow: hidden;
  display: grid;
  place-items: center;
}

.graph-canvas.loading {
  opacity: 0.72;
}

.graph-stage {
  position: relative;
  width: 100%;
  height: 100%;
  max-width: 100%;
  max-height: 100%;
}

.graph-edges {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.graph-edges line {
  stroke: color-mix(in srgb, var(--text-accent) 58%, var(--border-color));
  stroke-width: 0.28;
}

.graph-node {
  position: absolute;
  transform: translate(-50%, -50%);
  max-width: 126px;
  min-height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  padding: 0.32rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-main) 90%, var(--bg-sidebar));
  color: var(--text-primary);
  font: inherit;
  font-size: 0.72rem;
  font-weight: 650;
  line-height: 1.15;
  overflow-wrap: anywhere;
  cursor: pointer;
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.08);
}

.graph-node.focus {
  max-width: 180px;
  min-height: 54px;
  padding: 0.55rem 0.75rem;
  border-color: color-mix(in srgb, var(--text-accent) 65%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 14%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.86rem;
}

.graph-meta {
  flex-shrink: 0;
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0 0.8rem 0.65rem;
  color: var(--text-muted);
  font-size: 0.68rem;
}

.relation-list {
  flex: 0 0 215px;
  border-top: 1px solid var(--border-color);
  padding: 0.45rem;
}

.relation-row {
  display: grid;
  gap: 0.12rem;
  padding: 0.48rem 0.55rem;
}

.relation-row span {
  font-size: 0.8rem;
  font-weight: 650;
  overflow-wrap: anywhere;
}

.relation-row small {
  color: var(--text-muted);
  font-size: 0.66rem;
}

.viewer-header {
  padding: 0.75rem 0.85rem;
}

.viewer-header span {
  display: block;
  color: var(--text-secondary);
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
}

.viewer-header h2 {
  margin: 0.14rem 0;
  color: var(--text-primary);
  font-size: 1rem;
  line-height: 1.25;
  overflow-wrap: anywhere;
}

.viewer-header code {
  display: block;
  color: var(--text-muted);
  font-size: 0.68rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.markdown-viewer {
  flex: 1;
  min-height: 0;
  padding: 1rem 1.05rem 2rem;
  line-height: 1.55;
}

.markdown-viewer.loading {
  opacity: 0.72;
}

.md-line {
  min-height: 1.3rem;
  color: var(--text-primary);
  font-size: 0.88rem;
  overflow-wrap: anywhere;
}

.md-line.blank {
  height: 0.75rem;
}

.md-line.h1,
.md-line.h2,
.md-line.h3 {
  margin: 0.35rem 0 0.25rem;
  font-weight: 750;
  line-height: 1.25;
}

.md-line.h1 {
  font-size: 1.24rem;
}

.md-line.h2 {
  font-size: 1.04rem;
}

.md-line.h3 {
  font-size: 0.94rem;
}

.md-line.list {
  position: relative;
  display: block;
  padding-left: 1rem;
}

.md-line.quote {
  margin: 0.2rem 0;
  padding-left: 0.65rem;
  border-left: 2px solid color-mix(in srgb, var(--text-accent) 46%, var(--border-color));
  color: var(--text-secondary);
}

.md-line.code {
  padding: 0.25rem 0.4rem;
  border-radius: 4px;
  background: var(--bg-code);
  color: var(--text-code);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.78rem;
}

.bullet {
  position: absolute;
  left: 0;
  color: var(--text-muted);
}

.wikilink {
  display: inline;
  margin: 0;
  padding: 0 0.12rem;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  color: var(--text-accent);
  font: inherit;
  font-weight: 650;
  cursor: pointer;
}

.empty-viewer {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}

@media (prefers-color-scheme: dark) {
  .graph-node {
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.28);
  }
}

@media (max-width: 1120px) {
  .kb-header {
    grid-template-columns: 1fr auto;
  }

  .kb-search,
  .kb-stats {
    grid-column: 1 / -1;
  }

  .kb-stats {
    justify-content: flex-start;
  }

  .kb-layout {
    grid-template-columns: minmax(230px, 300px) minmax(0, 1fr);
  }
}

@media (max-width: 760px) {
  .kb-layout {
    grid-template-columns: 1fr;
    overflow-y: auto;
  }

  .tree-pane {
    min-height: 260px;
  }

  .workspace-pane {
    min-height: 620px;
  }

  .workspace-header {
    grid-template-columns: 1fr;
    justify-items: start;
  }

  .workspace-meta {
    justify-items: start;
  }

  .graph-canvas {
    min-height: 360px;
  }
}
</style>
