<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useMcpToolDiscovery } from '../composables/useMcpToolDiscovery';
import {
  buildToolPaletteEntries,
  getPaletteEntrySearchFields,
  scoredSearch,
  type ToolPaletteEntry,
} from '../lib/tool-palette-catalog';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import type { ServerHealthStatus } from '../lib/types';
import { useConfigStore } from '../stores/config';
import { useNodePaletteStore } from '../stores/node-palette';

interface SubmitNode {
  (type: 'workflow', fields: PipelineNodeDraftByType['workflow']): void;
  (type: 'prompt', fields: PipelineNodeDraftByType['prompt']): void;
  (type: 'sequentialThinking', fields: PipelineNodeDraftByType['sequentialThinking']): void;
  (type: 'tool', fields: PipelineNodeDraftByType['tool']): void;
}

const props = defineProps<{
  workflows: string[];
  submitNode: SubmitNode;
}>();

const configStore = useConfigStore();
const nodePaletteStore = useNodePaletteStore();
const {
  serverResults,
  loading,
  error,
  hasLoadedCatalog,
  refreshTools,
  hasStaleCacheByServer,
} = useMcpToolDiscovery();

function isToolEntry(
  entry: ToolPaletteEntry,
): entry is ToolPaletteEntry & { kind: 'tool'; toolName: string } {
  return entry.kind === 'tool' && typeof entry.toolName === 'string' && entry.toolName.length > 0;
}

const searchQuery = computed({
  get: () => nodePaletteStore.searchQuery,
  set: (value: string) => nodePaletteStore.setSearchQuery(value),
});
const isSearching = computed(() => searchQuery.value.trim().length > 0);
const firstAgent = computed(() => configStore.agentNames[0] ?? '');
const firstWorkflow = computed(() => props.workflows[0] ?? '');
const catalogEntries = computed(() =>
  buildToolPaletteEntries(serverResults.value, hasStaleCacheByServer.value),
);
const catalogTools = computed(() =>
  catalogEntries.value
    .filter(isToolEntry)
    .slice()
    .sort((left, right) =>
      left.server.localeCompare(right.server) || left.displayName.localeCompare(right.displayName)),
);
const recentToolEntries = computed(() =>
  nodePaletteStore.recentTools
    .map((toolName) => catalogTools.value.find((entry) => entry.toolName === toolName) ?? null)
    .filter((entry): entry is ToolPaletteEntry & { kind: 'tool'; toolName: string } => entry !== null),
);
const groupedTools = computed(() => {
  const groups = new Map<string, Array<ToolPaletteEntry & { kind: 'tool'; toolName: string }>>();
  for (const tool of catalogTools.value) {
    if (!groups.has(tool.server)) {
      groups.set(tool.server, []);
    }
    groups.get(tool.server)!.push(tool);
  }
  return [...groups.entries()].map(([server, tools]) => ({ server, tools }));
});
const searchResults = computed(() => {
  if (!isSearching.value) {
    return [];
  }
  return scoredSearch(
    catalogTools.value,
    searchQuery.value,
    getPaletteEntrySearchFields,
  ).map((result) => result.item);
});

watch(catalogTools, (tools) => {
  if (!hasLoadedCatalog.value) {
    return;
  }
  nodePaletteStore.pruneUnavailableTools(tools.map((tool) => tool.toolName));
}, { immediate: true });

onMounted(() => {
  if (!hasLoadedCatalog.value) {
    void refreshTools();
  }
});

function healthBadgeLabel(health: ServerHealthStatus): 'ready' | 'auth_required' | 'offline' {
  if (health === 'ready' || health === 'auth_required') {
    return health;
  }
  return 'offline';
}

function clearSearch(): void {
  nodePaletteStore.clearSearchQuery();
}

function refreshCatalog(): void {
  void refreshTools();
}

function addPromptNode(): void {
  if (!firstAgent.value) {
    return;
  }
  props.submitNode('prompt', {
    agent: firstAgent.value,
    prompt: 'Ask an agent',
  });
}

function addThinkingNode(): void {
  if (!firstAgent.value) {
    return;
  }
  props.submitNode('sequentialThinking', {
    agent: firstAgent.value,
    topic: 'Think through a problem',
    totalThoughts: 5,
  });
}

function addWorkflowNode(): void {
  if (!firstWorkflow.value) {
    return;
  }
  props.submitNode('workflow', {
    workflowId: firstWorkflow.value,
  });
}

function addToolNode(toolName: string): void {
  nodePaletteStore.setSelectedTool(toolName);
  nodePaletteStore.recordRecentTool(toolName);
  props.submitNode('tool', {
    toolName,
  });
}
</script>

<template>
  <section class="catalog-shell">
    <div class="catalog-header">
      <div>
        <h2>Add Node</h2>
        <p>Start with an intent, or browse the tool catalog.</p>
      </div>
      <button type="button" class="btn btn-ghost btn-xs" :disabled="loading" @click="refreshCatalog">
        {{ loading ? 'Refreshing…' : 'Refresh' }}
      </button>
    </div>

    <label class="search-field">
      <span class="sr-only">Search node catalog</span>
      <input
        v-model="searchQuery"
        class="field-input"
        type="search"
        placeholder="Search intents, tools, and servers…"
      />
    </label>

    <div class="catalog-status-row">
      <span>{{ isSearching ? `${searchResults.length} results` : 'Intent-first catalog' }}</span>
      <button v-if="searchQuery" type="button" class="btn btn-ghost btn-xs" @click="clearSearch">
        Clear
      </button>
    </div>

    <p v-if="error && !catalogTools.length" class="catalog-status catalog-status--error">
      {{ error }}
    </p>

    <template v-if="isSearching">
      <div v-if="searchResults.length" class="result-list" role="list">
        <button
          v-for="tool in searchResults"
          :key="tool.key"
          type="button"
          class="catalog-row"
          :class="{ 'is-selected': nodePaletteStore.selectedToolName === tool.toolName }"
          @click="addToolNode(tool.toolName)"
        >
          <div class="catalog-row__header">
            <div class="catalog-row__title">
              <strong>{{ tool.displayName }}</strong>
              <span class="server-tag">{{ tool.server }}</span>
            </div>
            <div class="catalog-row__actions">
              <span class="health-badge" :class="`health-badge--${healthBadgeLabel(tool.health)}`">
                {{ healthBadgeLabel(tool.health) }}
              </span>
              <button
                type="button"
                class="pin-button"
                :aria-pressed="nodePaletteStore.isToolPinned(tool.toolName)"
                @click.stop="nodePaletteStore.togglePinnedTool(tool.toolName)"
              >
                {{ nodePaletteStore.isToolPinned(tool.toolName) ? 'Unpin' : 'Pin' }}
              </button>
            </div>
          </div>
          <span class="catalog-row__description">{{ tool.description || 'No description.' }}</span>
        </button>
      </div>
      <p v-else class="catalog-status">No tools match “{{ searchQuery }}”.</p>
    </template>

    <template v-else>
      <section v-if="recentToolEntries.length" class="catalog-section">
        <div class="section-heading">
          <strong>Recent Tools</strong>
          <span>Most recently added</span>
        </div>
        <div class="chip-row">
          <button
            v-for="tool in recentToolEntries"
            :key="`recent-${tool.toolName}`"
            type="button"
            class="catalog-chip"
            @click="addToolNode(tool.toolName)"
          >
            {{ tool.displayName }}
          </button>
        </div>
      </section>

      <section class="catalog-section">
        <div class="section-heading">
          <strong>Common intents</strong>
          <span>Defaults match the canvas context menu.</span>
        </div>

        <div class="intent-grid">
          <button type="button" class="intent-card" :disabled="!firstAgent" @click="addPromptNode">
            <strong>Ask an Agent</strong>
            <p>Create a prompt node with the default agent and starter prompt.</p>
            <span class="intent-hint">Requires: agent, prompt</span>
          </button>

          <button type="button" class="intent-card" :disabled="!firstAgent" @click="addThinkingNode">
            <strong>Think Through a Problem</strong>
            <p>Create a sequential thinking node with a five-thought default.</p>
            <span class="intent-hint">Requires: agent, topic, totalThoughts</span>
          </button>

          <button type="button" class="intent-card" :disabled="!firstWorkflow" @click="addWorkflowNode">
            <strong>Run a Workflow</strong>
            <p>Create a workflow node using the first available workflow.</p>
            <span class="intent-hint">Requires: workflowId</span>
          </button>
        </div>
      </section>

      <details class="catalog-section tools-section" open>
        <summary class="section-summary">
          <span>
            <strong>Tools</strong>
            <small>Browse by server</small>
          </span>
          <span class="section-count">{{ catalogTools.length }}</span>
        </summary>

        <div v-if="groupedTools.length" class="server-groups">
          <section v-for="group in groupedTools" :key="group.server" class="server-group">
            <div class="server-group__header">
              <strong>{{ group.server }}</strong>
              <span>{{ group.tools.length }} tools</span>
            </div>

            <div class="server-group__list">
              <button
                v-for="tool in group.tools"
                :key="tool.key"
                type="button"
                class="catalog-row"
                :class="{ 'is-selected': nodePaletteStore.selectedToolName === tool.toolName }"
                @click="addToolNode(tool.toolName)"
              >
                <div class="catalog-row__header">
                  <div class="catalog-row__title">
                    <strong>{{ tool.displayName }}</strong>
                    <span class="server-tag">{{ tool.server }}</span>
                  </div>
                  <div class="catalog-row__actions">
                    <span class="health-badge" :class="`health-badge--${healthBadgeLabel(tool.health)}`">
                      {{ healthBadgeLabel(tool.health) }}
                    </span>
                    <button
                      type="button"
                      class="pin-button"
                      :aria-pressed="nodePaletteStore.isToolPinned(tool.toolName)"
                      @click.stop="nodePaletteStore.togglePinnedTool(tool.toolName)"
                    >
                      {{ nodePaletteStore.isToolPinned(tool.toolName) ? 'Unpin' : 'Pin' }}
                    </button>
                  </div>
                </div>
                <span class="catalog-row__description">{{ tool.description || 'No description.' }}</span>
              </button>
            </div>
          </section>
        </div>

        <p v-else class="catalog-status">
          {{ hasLoadedCatalog ? 'No tools are available yet.' : 'Refresh to load tools from MCP servers.' }}
        </p>
      </details>
    </template>
  </section>
</template>

<style scoped>
.catalog-shell {
  display: grid;
  gap: 1rem;
  align-content: start;
  height: 100%;
  padding: 1rem;
  border-right: 1px solid var(--border-default);
  background: color-mix(in srgb, var(--bg-main) 96%, transparent);
  overflow-y: auto;
}

.catalog-header,
.catalog-status-row,
.catalog-row__header,
.section-summary,
.server-group__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.catalog-header h2,
.catalog-header p,
.section-heading span,
.intent-card p,
.intent-hint,
.catalog-status,
.catalog-row__description,
.server-group__header span,
.section-summary small {
  margin: 0;
}

.catalog-header h2 {
  font-size: var(--text-lg);
}

.catalog-header p,
.catalog-status,
.section-heading span,
.intent-card p,
.intent-hint,
.server-group__header span,
.section-summary small {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.catalog-status--error {
  color: var(--color-status-abandoned);
}

.search-field {
  display: grid;
}

.catalog-section,
.server-groups,
.server-group,
.server-group__list,
.result-list {
  display: grid;
  gap: 0.75rem;
}

.section-heading {
  display: grid;
  gap: 0.15rem;
}

.chip-row,
.catalog-row__title,
.catalog-row__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}

.catalog-chip,
.server-tag,
.health-badge,
.pin-button,
.section-count {
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--border-default);
  border-radius: 999px;
  font-size: var(--text-xs);
}

.catalog-chip,
.pin-button {
  padding: 0.35rem 0.6rem;
  background: var(--bg-main);
  color: var(--text-primary);
}

.catalog-chip:hover,
.pin-button:hover,
.intent-card:hover,
.catalog-row:hover {
  background: color-mix(in srgb, var(--text-primary) 5%, var(--bg-main));
}

.intent-grid {
  display: grid;
  gap: 0.75rem;
}

.intent-card,
.catalog-row {
  display: grid;
  gap: 0.4rem;
  padding: 0.85rem 0.9rem;
  border: 1px solid var(--border-default);
  border-radius: 0.9rem;
  background: var(--bg-main);
  color: var(--text-primary);
  text-align: left;
  transition: background 120ms ease, border-color 120ms ease;
}

.intent-card:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.catalog-row.is-selected,
.intent-card:focus-visible,
.catalog-row:focus-visible,
.catalog-chip:focus-visible,
.pin-button:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

.tools-section {
  border: 1px solid var(--border-default);
  border-radius: 0.95rem;
  background: color-mix(in srgb, var(--bg-panel) 72%, transparent);
}

.section-summary {
  padding: 0.85rem 0.95rem;
  cursor: pointer;
  list-style: none;
}

.section-summary::-webkit-details-marker {
  display: none;
}

.section-count,
.server-tag {
  padding: 0.15rem 0.45rem;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-primary) 7%, transparent);
}

.server-groups {
  padding: 0 0.95rem 0.95rem;
}

.server-group + .server-group {
  padding-top: 0.75rem;
  border-top: 1px solid color-mix(in srgb, var(--border-default) 65%, transparent);
}

.catalog-row__description {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.health-badge {
  padding: 0.15rem 0.45rem;
  text-transform: lowercase;
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  color: var(--text-secondary);
}

.health-badge--ready {
  border-color: color-mix(in srgb, var(--color-status-completed) 35%, var(--border-default));
  color: var(--text-primary);
}

.health-badge--auth_required {
  border-color: color-mix(in srgb, var(--color-status-active) 35%, var(--border-default));
  color: var(--color-status-active);
}

.health-badge--offline {
  border-color: color-mix(in srgb, var(--color-status-abandoned) 35%, var(--border-default));
  color: var(--color-status-abandoned);
}

.pin-button {
  cursor: pointer;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
