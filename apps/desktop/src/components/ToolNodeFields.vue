<script setup lang="ts">
import { computed, ref, watch } from 'vue';
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
import { resolveToolDefinition } from '../lib/tool-categories';
import type { McpDiscoveredTool } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';
import SchemaForm from './SchemaForm.vue';

interface ToolPickerSection {
  server: string; status: 'ok' | 'error'; error?: string | null;
  tools: Array<McpDiscoveredTool & { namespacedName: string; displayName: string }>;
}
const props = withDefaults(defineProps<{
  active?: boolean;
  toolName: string;
  agentName?: string;
  agentConfig?: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
}>(), {
  active: false,
  agentName: '',
  agentConfig: () => ({}),
});
const emit = defineEmits<{
  'update:toolName': [value: string];
  'update:agentName': [value: string];
  'update:agentConfig': [value: PipelineNodeAgentConfig];
  'update:toolArgs': [value: Record<string, unknown>];
  validation: [state: { valid: boolean }];
}>();
const pipelineStore = usePipelineStore();
const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();
const { serverResults, loading, error, refreshTools } = useMcpToolDiscovery();
const refreshing = ref(false);
const attemptedInitialLoad = ref(false);
const searchQuery = ref('');
const expandedServers = ref(new Set<string>());
const sections = computed<ToolPickerSection[]>(() =>
  serverResults.value.map((result) => ({
    server: result.server,
    status: result.status,
    error: result.error,
    tools: [...result.tools]
      .sort((left, right) => left.name.localeCompare(right.name))
      .map((tool) => ({
        ...tool,
        // Backend already returns names as "server.toolName" — do NOT re-prefix.
        namespacedName: tool.name,
        displayName: tool.name.startsWith(`${result.server}.`)
          ? tool.name.slice(result.server.length + 1)
          : tool.name,
      })),
  })),
);
const filteredSections = computed<ToolPickerSection[]>(() => {
  const q = searchQuery.value.toLowerCase().trim();
  if (!q) return sections.value;
  return sections.value
    .map((s) => ({
      ...s,
      tools: s.tools.filter(
        (t) =>
          t.displayName.toLowerCase().includes(q) ||
          (t.description ?? '').toLowerCase().includes(q),
      ),
    }))
    .filter((s) => s.tools.length > 0 || s.status === 'error');
});
const selectedDiscoveryTool = computed<McpDiscoveredTool | null>(() => {
  for (const section of sections.value) {
    const match = section.tools.find((t) => t.namespacedName === props.toolName);
    if (match) return match;
  }
  return null;
});
const selectedFallbackTool = computed(() =>
  resolveToolDefinition(pipelineStore.tools, props.toolName) ?? null,
);
const selectedToolSchema = computed(
  () => selectedDiscoveryTool.value?.inputSchema ?? selectedFallbackTool.value?.inputSchema ?? null,
);
const hasCatalog = computed(() => sections.value.length > 0);
const isBusy = computed(() => loading.value || refreshing.value);
const isSearching = computed(() => searchQuery.value.trim().length > 0);
function toggleServer(server: string): void {
  const next = new Set(expandedServers.value);
  if (next.has(server)) next.delete(server);
  else next.add(server);
  expandedServers.value = next;
}
function isExpanded(server: string): boolean {
  return expandedServers.value.has(server) || isSearching.value;
}
async function refreshCatalog(): Promise<void> {
  refreshing.value = true;
  try {
    await Promise.allSettled([
      refreshTools(),
      pipelineStore.tools.length > 0 ? Promise.resolve(pipelineStore.tools) : pipelineStore.refreshTools(),
    ]);
  } finally {
    refreshing.value = false;
    attemptedInitialLoad.value = true;
  }
}
function selectTool(name: string): void {
  emit('update:toolName', name);
}
function updateToolName(event: Event): void {
  emit('update:toolName', (event.target as HTMLInputElement).value);
}
function onToolArgsChange(data: Record<string, unknown>): void {
  emit('update:toolArgs', data);
}
function onToolArgsValidation(state: { valid: boolean }): void {
  emit('validation', state);
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
function readyProbe(): AgentProbeResult | undefined {
  const probe = selectedAgentProbe();
  if (probe?.state === 'ready' && Object.keys(props.agentConfig ?? {}).length === 0) {
    emit('update:agentConfig', buildAgentConfigDefaults(probe) ?? {});
  }
  return probe?.state === 'ready' ? probe : undefined;
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
    setAgentConfigOptionValue(props.agentConfig, option, (event.target as HTMLSelectElement).value)
      ?? {},
  );
}
watch(
  () => props.active,
  (active) => {
    if (!active || attemptedInitialLoad.value || hasCatalog.value) return;
    void refreshCatalog();
  },
  { immediate: true },
);
/* Auto-expand newly discovered servers; preserve user-collapsed state. */
watch(sections, (next, prev) => {
  const known = new Set((prev ?? []).map((s) => s.server));
  const expanded = new Set(expandedServers.value);
  for (const s of next) {
    if (!known.has(s.server)) expanded.add(s.server);
  }
  expandedServers.value = expanded;
}, { immediate: true });
</script>

<template>
  <div class="tool-picker field-wide">
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
      <div v-else class="tool-agent-status" :class="selectedAgentProbe()?.state ?? 'pending'">
        <span class="status-pill">{{ selectedAgentProbe()?.state === 'offline' ? 'Offline' : selectedAgentProbe()?.state === 'error' ? 'Probe failed' : 'Pending' }}</span>
        <span>{{ selectedAgentProbe()?.error ?? 'Waiting for agent capability probe.' }}</span>
      </div>
    </div>
    <div class="tool-picker__header">
      <div>
        <span>Tool Picker</span>
        <p>Select an MCP tool by server, or type a name manually below.</p>
      </div>
      <button type="button" class="btn btn-ghost" :disabled="isBusy" @click="refreshCatalog">
        {{ isBusy ? 'Refreshing…' : 'Refresh' }}
      </button>
    </div>
    <p v-if="error && !hasCatalog" class="tool-picker__status tool-picker__status--error">{{ error }}</p>
    <template v-if="hasCatalog">
      <input
        v-model="searchQuery"
        class="field-input tool-picker__search"
        type="search"
        placeholder="Search tools by name or description…"
      />
      <div class="tool-picker__accordion">
        <section
          v-for="section in filteredSections"
          :key="section.server"
          class="tool-picker__server"
          :class="{ 'tool-picker__server--error': section.status === 'error' }"
        >
          <button
            type="button"
            class="tool-picker__server-toggle"
            :aria-expanded="isExpanded(section.server)"
            @click="toggleServer(section.server)"
          >
            <span class="tool-picker__chevron" :class="{ 'is-open': isExpanded(section.server) }">▸</span>
            <strong>{{ section.server }}</strong>
            <small>{{ section.tools.length }} tool{{ section.tools.length === 1 ? '' : 's' }}</small>
            <span :class="['tool-picker__badge', `tool-picker__badge--${section.status}`]">{{ section.status }}</span>
          </button>
          <p v-if="section.error && isExpanded(section.server)" class="tool-picker__server-error">
            {{ section.error }}<span v-if="section.tools.length > 0"> Showing last known tools.</span>
          </p>
          <div v-if="isExpanded(section.server) && section.tools.length > 0" class="tool-picker__tool-list">
            <button
              v-for="tool in section.tools"
              :key="tool.namespacedName"
              type="button"
              class="tool-picker__tool"
              :class="{ 'is-selected': tool.namespacedName === toolName }"
              @click="selectTool(tool.namespacedName)"
            >
              <div class="tool-picker__tool-head">
                <strong>{{ tool.displayName }}</strong>
                <span class="tool-picker__tool-server">{{ section.server }}</span>
              </div>
              <span class="tool-picker__tool-desc">{{ tool.description || 'No description.' }}</span>
            </button>
          </div>
          <p v-else-if="isExpanded(section.server)" class="tool-picker__status">No tools from this server.</p>
        </section>
      </div>
      <p v-if="isSearching && filteredSections.length === 0" class="tool-picker__status">
        No tools match "{{ searchQuery }}".
      </p>
    </template>
    <label>
      <span>Tool Name</span>
      <input class="field-input" :value="toolName" type="text" placeholder="server.ToolName" @input="updateToolName" />
    </label>
    <div v-if="toolName" class="field-block">
      <div class="tool-picker__args-label"><span>Tool Args</span><code>{{ toolName }}</code></div>
      <SchemaForm
        :schema="selectedToolSchema"
        :data="toolArgs"
        @change="onToolArgsChange"
        @validation="onToolArgsValidation"
      />
    </div>
  </div>
</template>

<style scoped>
.tool-picker, .field-block { display: grid; gap: 0.75rem; }
.tool-agent-panel {
  display: grid;
  gap: 0.65rem;
  padding: 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  background: color-mix(in srgb, var(--bg-sidebar) 72%, transparent);
}
.tool-agent-grid {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
}
.agent-mode-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.mode-badge,.status-pill {
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
.mode-badge.selected {
  border-color: var(--border-focus);
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-user) 45%, transparent);
}
.tool-agent-status {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  color: var(--text-secondary);
}
.status-pill {
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.tool-picker__header {
  display: flex; align-items: flex-start; justify-content: space-between; gap: 0.75rem;
}
.tool-picker__header p, .tool-picker__status, .tool-picker__tool-desc, .field-block {
  color: var(--text-secondary); font-size: var(--text-sm);
}
.tool-picker__status--error, .tool-picker__server-error {
  color: var(--color-status-abandoned); font-size: var(--text-sm);
}
.tool-picker__server-error { padding: 0 0.75rem 0.5rem 2rem; }
.tool-picker__accordion {
  display: flex; flex-direction: column; gap: 0.25rem;
  max-height: min(28rem, 50vh); overflow-y: auto;
  scrollbar-width: thin; scrollbar-color: var(--border-default) transparent;
}
.tool-picker__server {
  border: 1px solid var(--border-color); border-radius: 0.75rem;
  background: var(--bg-main); overflow: hidden;
}
.tool-picker__server--error {
  border-color: color-mix(in srgb, var(--color-status-abandoned) 45%, var(--border-color));
}
.tool-picker__server-toggle {
  display: flex; align-items: center; gap: 0.5rem;
  width: 100%; padding: 0.6rem 0.75rem;
  border: none; background: none; color: var(--text-primary);
  font-size: var(--text-sm); cursor: pointer; text-align: left;
}
.tool-picker__server-toggle:hover {
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
}
.tool-picker__server-toggle small { color: var(--text-secondary); margin-left: auto; }
.tool-picker__chevron {
  display: inline-block; transition: transform 0.15s ease; font-size: 0.7rem; flex-shrink: 0;
}
.tool-picker__chevron.is-open { transform: rotate(90deg); }
.tool-picker__badge {
  padding: 0.1rem 0.45rem; border-radius: 999px;
  font-size: var(--text-xs); text-transform: uppercase; flex-shrink: 0;
}
.tool-picker__badge--ok {
  background: color-mix(in srgb, var(--color-status-completed) 18%, transparent);
  color: var(--color-status-completed);
}
.tool-picker__badge--error {
  background: color-mix(in srgb, var(--color-status-abandoned) 18%, transparent);
  color: var(--color-status-abandoned);
}
.tool-picker__tool-list {
  display: flex; flex-direction: column; gap: 0.35rem; padding: 0 0.5rem 0.5rem;
}
.tool-picker__tool {
  display: grid; gap: 0.2rem; padding: 0.5rem 0.65rem;
  border: 1px solid var(--border-color); border-radius: 0.5rem;
  background: var(--bg-main); color: var(--text-primary);
  text-align: left; cursor: pointer;
  transition: border-color 0.1s, box-shadow 0.1s, background 0.1s;
}
.tool-picker__tool:hover {
  background: color-mix(in srgb, var(--text-primary) 4%, var(--bg-main));
  border-color: var(--text-secondary);
}
.tool-picker__tool.is-selected {
  border-color: var(--border-focus); box-shadow: 0 0 0 1px var(--border-focus);
}
.tool-picker__tool-head {
  display: flex; align-items: center; justify-content: space-between; gap: 0.5rem;
}
.tool-picker__tool-server {
  padding: 0.05rem 0.4rem; border-radius: 999px;
  background: color-mix(in srgb, var(--text-primary) 8%, transparent);
  color: var(--text-secondary); font-size: var(--text-xs); flex-shrink: 0; white-space: nowrap;
}
.tool-picker__tool-desc {
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
}
.tool-picker__args-label {
  display: flex; align-items: center; justify-content: space-between; gap: 0.5rem;
}
.tool-picker__args-label code { font-size: var(--text-xs); color: var(--text-secondary); }
</style>
