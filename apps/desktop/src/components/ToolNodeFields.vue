<script setup lang="ts">
import { computed, watch } from 'vue';
import type { AgentProbeResult } from '../lib/agent-probe-types';
import { useMcpToolDiscovery } from '../composables/useMcpToolDiscovery';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import {
  buildAgentConfigDefaults,
  formatPipelineAgentOptionLabel,
  getAgentConfigOptionValue,
  getAgentModelValue,
  getAgentThoughtLevelValue,
  getEditableAgentConfigOptions,
  getAgentModeValue,
  isSessionConfigSelectGroup,
  setAgentConfigOptionValue,
  setAgentModelValue,
  setAgentModeValue,
  setAgentThoughtLevelValue,
} from '../composables/pipeline-node-form-helpers';
import { getBareToolName, resolveToolDefinition } from '../lib/tool-categories';
import type { McpDiscoveredTool } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { useNodePaletteStore } from '../stores/node-palette';
import { usePipelineStore } from '../stores/pipeline';
import AgentProbePreview from './AgentProbePreview.vue';
import SchemaForm from './SchemaForm.vue';

interface ToolPaletteItem extends McpDiscoveredTool {
  server: string;
  status: 'ok' | 'error';
  error?: string | null;
  namespacedName: string;
  displayName: string;
}

interface HighlightPart {
  text: string;
  match: boolean;
}

const props = withDefaults(defineProps<{
  active?: boolean;
  toolName: string;
  agentName?: string;
  agentConfig?: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  canSubmit?: boolean;
}>(), {
  active: false,
  agentName: '',
  agentConfig: () => ({}),
  canSubmit: false,
});

const emit = defineEmits<{
  'update:toolName': [value: string];
  'update:agentName': [value: string];
  'update:agentConfig': [value: PipelineNodeAgentConfig];
  'update:toolArgs': [value: Record<string, unknown>];
  validation: [state: { valid: boolean }];
  submitRequest: [];
}>();

const pipelineStore = usePipelineStore();
const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();
const nodePaletteStore = useNodePaletteStore();
const { serverResults, loading, error, hasLoadedCatalog, refreshTools } = useMcpToolDiscovery();

const sections = computed(() =>
  serverResults.value.map((result) => ({
    server: result.server,
    status: result.status,
    error: result.error,
    tools: [...result.tools]
      .sort((left, right) => left.name.localeCompare(right.name))
      .map((tool) => ({
        ...tool,
        server: result.server,
        status: result.status,
        error: result.error,
        namespacedName: tool.name,
        displayName: tool.name.startsWith(`${result.server}.`)
          ? tool.name.slice(result.server.length + 1)
          : getBareToolName(tool.name),
      })),
  })),
);

const catalogTools = computed<ToolPaletteItem[]>(() =>
  sections.value.flatMap((section) => section.tools),
);

const hasCatalog = computed(() => catalogTools.value.length > 0);
const searchQuery = computed({
  get: () => nodePaletteStore.searchQuery,
  set: (value: string) => nodePaletteStore.setSearchQuery(value),
});
const isSearching = computed(() => searchQuery.value.trim().length > 0);
const selectedDiscoveryTool = computed<ToolPaletteItem | null>(() =>
  catalogTools.value.find((tool) => tool.namespacedName === props.toolName) ?? null,
);
const selectedFallbackTool = computed(() =>
  resolveToolDefinition(pipelineStore.tools, props.toolName) ?? null,
);
const selectedToolSchema = computed(
  () => selectedDiscoveryTool.value?.inputSchema ?? selectedFallbackTool.value?.inputSchema ?? null,
);
const selectedToolDescription = computed(
  () => selectedDiscoveryTool.value?.description ?? selectedFallbackTool.value?.description ?? null,
);
const requiredArgNames = computed(() => {
  const schema = selectedToolSchema.value;
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    return [];
  }

  const required = (schema as { required?: unknown }).required;
  return Array.isArray(required)
    ? required.filter((entry): entry is string => typeof entry === 'string')
    : [];
});

const quickAddTools = computed<ToolPaletteItem[]>(() => {
  const orderedNames = [
    ...nodePaletteStore.pinnedTools,
    ...nodePaletteStore.recentTools.filter((name) => !nodePaletteStore.pinnedTools.includes(name)),
  ];

  return orderedNames
    .map((name) => catalogTools.value.find((tool) => tool.namespacedName === name) ?? null)
    .filter((tool): tool is ToolPaletteItem => tool !== null);
});

const searchResults = computed<ToolPaletteItem[]>(() => {
  const query = searchQuery.value.toLowerCase().trim();
  if (!query) {
    return [];
  }

  return catalogTools.value.filter((tool) =>
    tool.displayName.toLowerCase().includes(query)
    || tool.namespacedName.toLowerCase().includes(query)
    || (tool.description ?? '').toLowerCase().includes(query),
  );
});

const visibleTools = computed<ToolPaletteItem[]>(() =>
  isSearching.value ? searchResults.value : quickAddTools.value,
);

const activeDescendantId = computed(() =>
  visibleTools.value.some((tool) => tool.namespacedName === nodePaletteStore.selectedToolName)
    ? optionId(nodePaletteStore.selectedToolName!)
    : undefined,
);

function optionId(name: string): string {
  return `tool-palette-option-${name.replace(/[^a-z0-9_-]+/gi, '-')}`;
}

function getProbe(agentName: string): AgentProbeResult | undefined {
  return agentName ? capabilitiesStore.getProbe(agentName) : undefined;
}

function formatAgentOptionLabel(agentName: string): string {
  return formatPipelineAgentOptionLabel(agentName, getProbe(agentName));
}

function selectedAgentProbe(): AgentProbeResult | undefined {
  return props.agentName ? getProbe(props.agentName) : undefined;
}

function isProbeLoading(agentName: string): boolean {
  return capabilitiesStore.probeStatus === 'probing' && !getProbe(agentName);
}

function readyProbe(): AgentProbeResult | undefined {
  const probe = selectedAgentProbe();
  if (probe?.state === 'ready' && Object.keys(props.agentConfig ?? {}).length === 0) {
    emit('update:agentConfig', buildAgentConfigDefaults(probe) ?? {});
  }
  return probe?.state === 'ready' ? probe : undefined;
}

async function refreshCatalog(): Promise<void> {
  await Promise.allSettled([
    refreshTools(),
    pipelineStore.tools.length > 0 ? Promise.resolve(pipelineStore.tools) : pipelineStore.refreshTools(),
  ]);
}

function selectTool(name: string): void {
  nodePaletteStore.setSelectedTool(name);
  emit('update:toolName', name);
}

function togglePin(toolName: string): void {
  nodePaletteStore.togglePinnedTool(toolName);
}

function clearSelection(): void {
  nodePaletteStore.setSelectedTool(null);
  emit('update:toolName', '');
  emit('update:toolArgs', {});
  emit('validation', { valid: true });
}

function clearSearch(): void {
  nodePaletteStore.clearSearchQuery();
}

function updateToolName(event: Event): void {
  selectTool((event.target as HTMLInputElement).value);
}

function onToolArgsChange(data: Record<string, unknown>): void {
  emit('update:toolArgs', data);
}

function onToolArgsValidation(state: { valid: boolean }): void {
  emit('validation', state);
}

function updateAgentName(event: Event): void {
  emit('update:agentName', (event.target as HTMLSelectElement).value);
  emit('update:agentConfig', {});
}

function updateAgentMode(modeId: string): void {
  emit('update:agentConfig', setAgentModeValue(props.agentConfig, modeId) ?? {});
}

function updateAgentModel(event: Event): void {
  emit(
    'update:agentConfig',
    setAgentModelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {},
  );
}

function updateAgentThoughtLevel(event: Event): void {
  emit(
    'update:agentConfig',
    setAgentThoughtLevelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {},
  );
}

function updateAgentConfigOption(option: ReturnType<typeof getEditableAgentConfigOptions>[number], event: Event): void {
  emit(
    'update:agentConfig',
    setAgentConfigOptionValue(props.agentConfig, option, (event.target as HTMLSelectElement).value) ?? {},
  );
}

function isSelected(toolName: string): boolean {
  return toolName === nodePaletteStore.selectedToolName;
}

function focusDelta(delta: number): void {
  if (visibleTools.value.length === 0) {
    return;
  }

  const currentIndex = visibleTools.value.findIndex(
    (tool) => tool.namespacedName === nodePaletteStore.selectedToolName,
  );
  const nextIndex = currentIndex === -1
    ? (delta > 0 ? 0 : visibleTools.value.length - 1)
    : (currentIndex + delta + visibleTools.value.length) % visibleTools.value.length;
  selectTool(visibleTools.value[nextIndex]!.namespacedName);
}

function submitSelectedTool(): void {
  if (props.canSubmit) {
    emit('submitRequest');
  }
}

function handlePaletteKeydown(event: KeyboardEvent): void {
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      focusDelta(1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      focusDelta(-1);
      break;
    case 'Enter':
    case ' ':
      if (visibleTools.value.length === 0) {
        return;
      }

      event.preventDefault();
      if (!nodePaletteStore.selectedToolName) {
        selectTool(visibleTools.value[0]!.namespacedName);
        return;
      }

      if (nodePaletteStore.selectedToolName !== props.toolName) {
        selectTool(nodePaletteStore.selectedToolName);
        return;
      }

      submitSelectedTool();
      break;
    case 'Escape':
      if (!searchQuery.value) {
        return;
      }

      event.preventDefault();
      clearSearch();
      break;
    default:
      break;
  }
}

function handleSearchKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape' || !searchQuery.value) {
    return;
  }

  event.preventDefault();
  clearSearch();
}

function highlightText(text: string, query: string): HighlightPart[] {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) {
    return [{ text, match: false }];
  }

  const normalizedText = text.toLowerCase();
  const parts: HighlightPart[] = [];
  let cursor = 0;

  while (cursor < text.length) {
    const matchIndex = normalizedText.indexOf(normalizedQuery, cursor);
    if (matchIndex === -1) {
      parts.push({ text: text.slice(cursor), match: false });
      break;
    }

    if (matchIndex > cursor) {
      parts.push({ text: text.slice(cursor, matchIndex), match: false });
    }

    parts.push({
      text: text.slice(matchIndex, matchIndex + normalizedQuery.length),
      match: true,
    });
    cursor = matchIndex + normalizedQuery.length;
  }

  return parts.length > 0 ? parts : [{ text, match: false }];
}

watch(
  () => props.active,
  (active) => {
    if (!active || hasCatalog.value) {
      return;
    }

    void refreshCatalog();
  },
  { immediate: true },
);

watch(
  catalogTools,
  (tools) => {
    if (!hasLoadedCatalog.value) {
      return;
    }

    nodePaletteStore.pruneUnavailableTools(tools.map((tool) => tool.namespacedName));
    if (!props.toolName && nodePaletteStore.selectedToolName) {
      emit('update:toolName', nodePaletteStore.selectedToolName);
    }
  },
  { immediate: true },
);

watch(
  () => props.toolName,
  (value) => {
    nodePaletteStore.setSelectedTool(value || null);
  },
  { immediate: true },
);

watch(
  visibleTools,
  (tools) => {
    if (tools.length === 0) {
      return;
    }

    if (!nodePaletteStore.selectedToolName) {
      selectTool(tools[0]!.namespacedName);
      return;
    }

    if (!tools.some((tool) => tool.namespacedName === nodePaletteStore.selectedToolName) && isSearching.value) {
      selectTool(tools[0]!.namespacedName);
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="tool-palette field-wide">
    <label>
      <span>Agent Override</span>
      <select
        class="field-input"
        data-testid="tool-agent-select"
        :value="agentName"
        @change="updateAgentName"
      >
        <option value="">Use tool default</option>
        <option v-for="name in configStore.agentNames" :key="name" :value="name">
          {{ formatAgentOptionLabel(name) }}
        </option>
      </select>
    </label>

    <div v-if="agentName" class="tool-agent-panel">
      <AgentProbePreview
        compact
        :agent-name="agentName"
        :probe="selectedAgentProbe()"
        :loading="isProbeLoading(agentName)"
      />
      <template v-if="readyProbe()">
        <div v-if="readyProbe()?.modeOptions.length" class="agent-mode-badges">
          <button
            v-for="mode in readyProbe()?.modeOptions"
            :key="mode.id"
            type="button"
            class="mode-badge"
            :class="{ selected: getAgentModeValue(agentConfig, readyProbe()) === mode.id }"
            :data-testid="`tool-mode-badge-${mode.id}`"
            @click="updateAgentMode(mode.id)"
          >
            {{ mode.name }}
          </button>
        </div>
        <div class="tool-agent-grid">
          <label v-if="readyProbe()?.modelOptions.length">
            <span>Model</span>
            <select
              class="field-input"
              data-testid="tool-model-select"
              :value="getAgentModelValue(agentConfig, readyProbe())"
              @change="updateAgentModel"
            >
              <option v-for="model in readyProbe()?.modelOptions" :key="model.modelId" :value="model.modelId">
                {{ model.name }}
              </option>
            </select>
          </label>
          <label v-if="readyProbe()?.thoughtLevelConfigOption">
            <span>{{ readyProbe()?.thoughtLevelConfigOption?.name }}</span>
            <select
              class="field-input"
              data-testid="tool-thought-level-select"
              :value="getAgentThoughtLevelValue(agentConfig, readyProbe())"
              @change="updateAgentThoughtLevel"
            >
              <template
                v-for="entry in readyProbe()?.thoughtLevelConfigOption?.options ?? []"
                :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
              >
                <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                  <option
                    v-for="groupOption in entry.options"
                    :key="groupOption.value"
                    :value="groupOption.value"
                  >
                    {{ groupOption.name }}
                  </option>
                </optgroup>
                <option v-else :value="entry.value">{{ entry.name }}</option>
              </template>
            </select>
          </label>
          <label
            v-for="option in getEditableAgentConfigOptions(readyProbe())"
            :key="option.id"
          >
            <span>{{ option.name }}</span>
            <select
              class="field-input"
              :data-testid="`tool-config-${option.id}`"
              :value="getAgentConfigOptionValue(agentConfig, option)"
              @change="updateAgentConfigOption(option, $event)"
            >
              <template
                v-for="entry in option.options"
                :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
              >
                <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                  <option
                    v-for="groupOption in entry.options"
                    :key="groupOption.value"
                    :value="groupOption.value"
                  >
                    {{ groupOption.name }}
                  </option>
                </optgroup>
                <option v-else :value="entry.value">{{ entry.name }}</option>
              </template>
            </select>
          </label>
        </div>
      </template>
    </div>

    <div class="tool-palette__header">
      <div>
        <span>Tool Palette</span>
        <p>Search across servers, or reuse your recent and pinned tools from Quick Add.</p>
      </div>
      <button type="button" class="btn btn-ghost" :disabled="loading" @click="refreshCatalog">
        {{ loading ? 'Refreshing…' : 'Refresh' }}
      </button>
    </div>

    <p v-if="error && !hasCatalog" class="tool-palette__status tool-palette__status--error">{{ error }}</p>

    <input
      v-model="searchQuery"
      class="field-input tool-palette__search"
      type="search"
      placeholder="Search tools by name or description…"
      aria-label="Search tools"
      @keydown="handleSearchKeydown"
    />

    <div class="tool-palette__status-row">
      <strong>{{ isSearching ? `${searchResults.length} results` : 'Quick Add' }}</strong>
      <button
        v-if="searchQuery"
        type="button"
        class="btn btn-ghost btn-xs"
        @click="clearSearch"
      >
        Clear search
      </button>
    </div>

    <div
      class="tool-palette__listbox"
      role="listbox"
      :aria-label="isSearching ? 'Tool search results' : 'Quick Add tools'"
      :aria-activedescendant="activeDescendantId"
      tabindex="0"
      @keydown="handlePaletteKeydown"
    >
      <p v-if="!hasCatalog" class="tool-palette__status">Refresh to load tools from MCP servers.</p>
      <p v-else-if="visibleTools.length === 0 && isSearching" class="tool-palette__status">
        No tools match “{{ searchQuery }}”.
      </p>
      <p v-else-if="visibleTools.length === 0" class="tool-palette__status">
        Quick Add is empty. Start typing to search across all connected servers.
      </p>
      <button
        v-for="tool in visibleTools"
        :id="optionId(tool.namespacedName)"
        :key="tool.namespacedName"
        type="button"
        class="tool-palette__option"
        :class="{ 'is-selected': isSelected(tool.namespacedName) }"
        role="option"
        :aria-selected="isSelected(tool.namespacedName)"
        @click="selectTool(tool.namespacedName)"
      >
        <div class="tool-palette__option-head">
          <strong>
            <template v-for="(part, index) in highlightText(tool.displayName, searchQuery)" :key="`${tool.namespacedName}-name-${index}`">
              <mark v-if="part.match">{{ part.text }}</mark>
              <span v-else>{{ part.text }}</span>
            </template>
          </strong>
          <span class="tool-palette__server">{{ tool.server }}</span>
        </div>
        <span class="tool-palette__option-desc">
          <template v-for="(part, index) in highlightText(tool.description || 'No description.', searchQuery)" :key="`${tool.namespacedName}-desc-${index}`">
            <mark v-if="part.match">{{ part.text }}</mark>
            <span v-else>{{ part.text }}</span>
          </template>
        </span>
        <span class="tool-palette__option-tags">
          <span v-if="nodePaletteStore.isToolPinned(tool.namespacedName)" class="tool-palette__tag">Pinned</span>
          <span v-if="nodePaletteStore.recentTools.includes(tool.namespacedName)" class="tool-palette__tag">Recent</span>
          <span v-if="tool.status === 'error'" class="tool-palette__tag tool-palette__tag--warning">Stale catalog</span>
        </span>
      </button>
    </div>

    <label>
      <span>Tool Name</span>
      <input
        class="field-input"
        :value="toolName"
        type="text"
        placeholder="server.ToolName"
        @input="updateToolName"
      />
    </label>

    <section v-if="toolName" class="tool-palette__detail">
      <div class="tool-palette__detail-head">
        <div>
          <strong>{{ selectedDiscoveryTool?.displayName ?? getBareToolName(toolName) }}</strong>
          <p>{{ selectedToolDescription || 'No description.' }}</p>
        </div>
        <div class="tool-palette__detail-actions">
          <span v-if="selectedDiscoveryTool?.server" class="tool-palette__server">
            {{ selectedDiscoveryTool.server }}
          </span>
          <button type="button" class="btn btn-ghost btn-xs" @click="togglePin(toolName)">
            {{ nodePaletteStore.isToolPinned(toolName) ? 'Unpin' : 'Pin' }}
          </button>
          <button type="button" class="btn btn-ghost btn-xs" @click="clearSelection">Clear</button>
        </div>
      </div>

      <div class="tool-palette__required">
        <span>Required args</span>
        <div class="tool-palette__required-tags">
          <span
            v-for="requiredName in requiredArgNames"
            :key="requiredName"
            class="tool-palette__required-tag"
          >
            {{ requiredName }}
          </span>
          <span v-if="requiredArgNames.length === 0" class="tool-palette__required-empty">
            No required args
          </span>
        </div>
      </div>

      <SchemaForm
        :schema="selectedToolSchema"
        :data="toolArgs"
        @change="onToolArgsChange"
        @validation="onToolArgsValidation"
      />

      <div class="tool-palette__detail-footer">
        <button type="button" class="btn btn-primary" :disabled="!canSubmit" @click="submitSelectedTool">
          Add Tool
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.tool-palette,
.tool-agent-panel,
.tool-agent-grid,
.tool-palette__detail {
  display: grid;
  gap: 0.75rem;
}

.tool-agent-panel,
.tool-palette__detail {
  padding: 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--bg-sidebar) 72%, transparent);
}

.tool-agent-grid {
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
}

.agent-mode-badges,
.tool-palette__option-tags,
.tool-palette__required-tags,
.tool-palette__detail-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  align-items: center;
}

.mode-badge,
.tool-palette__tag,
.tool-palette__required-tag,
.tool-palette__server {
  display: inline-flex;
  align-items: center;
  padding: 0.15rem 0.45rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  font-size: var(--text-xs);
}

.mode-badge {
  background: var(--bg-main);
  color: var(--text-secondary);
  cursor: pointer;
}

.mode-badge.selected,
.tool-palette__option.is-selected {
  border-color: var(--border-focus);
  box-shadow: 0 0 0 1px var(--border-focus);
}

.tool-palette__header,
.tool-palette__status-row,
.tool-palette__detail-head,
.tool-palette__detail-footer {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.tool-palette__header p,
.tool-palette__detail-head p,
.tool-palette__status,
.tool-palette__option-desc,
.tool-palette__required-empty {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.tool-palette__status--error,
.tool-palette__tag--warning {
  color: var(--color-status-abandoned);
}

.tool-palette__listbox {
  display: grid;
  gap: 0.45rem;
  max-height: min(18rem, 38vh);
  padding: 0.1rem;
  overflow-y: auto;
  border-radius: 0.8rem;
  outline: none;
}

.tool-palette__option {
  display: grid;
  gap: 0.35rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  background: var(--bg-main);
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
}

.tool-palette__option:hover {
  background: color-mix(in srgb, var(--text-primary) 4%, var(--bg-main));
}

.tool-palette__option:focus-visible,
.tool-palette__listbox:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

.tool-palette__option-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.5rem;
}

.tool-palette__option-desc {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.tool-palette__server {
  background: color-mix(in srgb, var(--text-primary) 8%, transparent);
  color: var(--text-secondary);
}

.tool-palette__tag {
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  color: var(--text-secondary);
}

.tool-palette__tag--warning {
  border-color: color-mix(in srgb, var(--color-status-abandoned) 35%, var(--border-color));
}

.tool-palette__required {
  display: grid;
  gap: 0.45rem;
}

.tool-palette__required-tag {
  background: color-mix(in srgb, var(--bg-main) 84%, transparent);
}

mark {
  padding: 0;
  background: color-mix(in srgb, var(--color-status-active) 24%, transparent);
  color: inherit;
}
</style>
