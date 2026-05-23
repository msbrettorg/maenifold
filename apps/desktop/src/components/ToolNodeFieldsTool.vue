<script setup lang="ts">
import { computed, watch } from 'vue';
import { storeToRefs } from 'pinia';
import type { AgentProbeResult } from '../lib/agent-probe-types-defs';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types-node';
import { buildAgentConfigDefaults, formatPipelineAgentOptionLabel, getAgentConfigOptionValue, getAgentModelValue, getAgentThoughtLevelValue, getEditableAgentConfigOptions, getAgentModeValue, isSessionConfigSelectGroup, setAgentConfigOptionValue, setAgentModelValue, setAgentModeValue, setAgentThoughtLevelValue } from '../composables/pipeline-node-form-helpers';
import { getBareToolName, resolveToolDefinition } from '../lib/tool-categories';
import type { McpDiscoveredTool } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { useNodePaletteStore } from '../stores/node-palette';
import { usePipelineStore } from '../stores/pipeline';
import ToolNodeFieldsPrompt from './ToolNodeFieldsPrompt.vue';
import ToolNodeFieldsThinking from './ToolNodeFieldsThinking.vue';
import ToolNodeFieldsWorkflow from './ToolNodeFieldsWorkflow.vue';

interface ToolPaletteItem extends McpDiscoveredTool { server: string; status: 'ok' | 'error'; error?: string | null; namespacedName: string; displayName: string; }
interface HighlightPart { text: string; match: boolean; }

const props = withDefaults(defineProps<{ active?: boolean; toolName: string; agentName?: string; agentConfig?: PipelineNodeAgentConfig; toolArgs: Record<string, unknown>; canSubmit?: boolean; }>(), { active: false, agentName: '', agentConfig: () => ({}), canSubmit: false });
const emit = defineEmits<{ 'update:toolName': [value: string]; 'update:agentName': [value: string]; 'update:agentConfig': [value: PipelineNodeAgentConfig]; 'update:toolArgs': [value: Record<string, unknown>]; validation: [state: { valid: boolean }]; submitRequest: []; }>();

const pipelineStore = usePipelineStore(), configStore = useConfigStore(), capabilitiesStore = useAgentCapabilitiesStore(), nodePaletteStore = useNodePaletteStore();
const mcpToolDiscoveryStore = useMcpToolDiscoveryStore();
const { serverResults, loading, error, hasLoadedCatalog } = storeToRefs(mcpToolDiscoveryStore);
const { refreshTools } = mcpToolDiscoveryStore;
const sections = computed(() => serverResults.value.map((result) => ({ server: result.server, status: result.status, error: result.error, tools: [...result.tools].sort((left, right) => left.name.localeCompare(right.name)).map((tool) => ({ ...tool, server: result.server, status: result.status, error: result.error, namespacedName: tool.name, displayName: tool.name.startsWith(`${result.server}.`) ? tool.name.slice(result.server.length + 1) : getBareToolName(tool.name) })) })));
const catalogTools = computed<ToolPaletteItem[]>(() => sections.value.flatMap((section) => section.tools));
const hasCatalog = computed(() => catalogTools.value.length > 0);
const searchQuery = computed({ get: () => nodePaletteStore.searchQuery, set: (value: string) => nodePaletteStore.setSearchQuery(value) });
const isSearching = computed(() => searchQuery.value.trim().length > 0);
const selectedDiscoveryTool = computed<ToolPaletteItem | null>(() => catalogTools.value.find((tool) => tool.namespacedName === props.toolName) ?? null);
const selectedFallbackTool = computed(() => resolveToolDefinition(pipelineStore.tools, props.toolName) ?? null);
const selectedToolSchema = computed(() => selectedDiscoveryTool.value?.inputSchema ?? selectedFallbackTool.value?.inputSchema ?? null);
const selectedToolDescription = computed(() => selectedDiscoveryTool.value?.description ?? selectedFallbackTool.value?.description ?? null);
const requiredArgNames = computed(() => { const schema = selectedToolSchema.value; if (!schema || typeof schema !== 'object' || Array.isArray(schema)) return []; const required = (schema as { required?: unknown }).required; return Array.isArray(required) ? required.filter((entry): entry is string => typeof entry === 'string') : []; });
const quickAddTools = computed<ToolPaletteItem[]>(() => [...nodePaletteStore.pinnedTools, ...nodePaletteStore.recentTools.filter((name) => !nodePaletteStore.pinnedTools.includes(name))].map((name) => catalogTools.value.find((tool) => tool.namespacedName === name) ?? null).filter((tool): tool is ToolPaletteItem => tool !== null));
const searchResults = computed<ToolPaletteItem[]>(() => { const query = searchQuery.value.toLowerCase().trim(); return query ? catalogTools.value.filter((tool) => tool.displayName.toLowerCase().includes(query) || tool.namespacedName.toLowerCase().includes(query) || (tool.description ?? '').toLowerCase().includes(query)) : []; });
const visibleTools = computed<ToolPaletteItem[]>(() => isSearching.value ? searchResults.value : quickAddTools.value);
const activeDescendantId = computed(() => visibleTools.value.some((tool) => tool.namespacedName === nodePaletteStore.selectedToolName) ? optionId(nodePaletteStore.selectedToolName!) : undefined);
const selectedAgentProbeResult = computed(() => props.agentName ? getProbe(props.agentName) : undefined);
const readyProbeResult = computed(() => readyProbe());
const editableAgentConfigOptions = computed(() => getEditableAgentConfigOptions(readyProbeResult.value));
const selectedToolDisplayName = computed(() => selectedDiscoveryTool.value?.displayName ?? getBareToolName(props.toolName));
const selectedToolServer = computed(() => selectedDiscoveryTool.value?.server ?? null);

function optionId(name: string): string { return `tool-palette-option-${name.replace(/[^a-z0-9_-]+/gi, '-')}`; }
function getProbe(agentName: string): AgentProbeResult | undefined { return agentName ? capabilitiesStore.getProbe(agentName) : undefined; }
function formatAgentOptionLabel(agentName: string): string { return formatPipelineAgentOptionLabel(agentName, getProbe(agentName)); }
function isProbeLoading(agentName: string): boolean { return capabilitiesStore.probeStatus === 'probing' && !getProbe(agentName); }
function readyProbe(): AgentProbeResult | undefined { const probe = selectedAgentProbeResult.value; if (probe?.state === 'ready' && Object.keys(props.agentConfig ?? {}).length === 0) emit('update:agentConfig', buildAgentConfigDefaults(probe) ?? {}); return probe?.state === 'ready' ? probe : undefined; }
async function refreshCatalogSafe(): Promise<void> { await Promise.allSettled([refreshTools(), pipelineStore.tools.length > 0 ? Promise.resolve(pipelineStore.tools) : pipelineStore.refreshTools()]); }
function selectTool(name: string): void { nodePaletteStore.setSelectedTool(name); emit('update:toolName', name); }
function clearSelection(): void { nodePaletteStore.setSelectedTool(null); emit('update:toolName', ''); emit('update:toolArgs', {}); emit('validation', { valid: true }); }
function clearSearch(): void { nodePaletteStore.clearSearchQuery(); }
function updateToolName(event: Event): void { selectTool((event.target as HTMLInputElement).value); }
function onToolArgsChange(data: Record<string, unknown>): void { emit('update:toolArgs', data); }
function onToolArgsValidation(state: { valid: boolean }): void { emit('validation', state); }
function updateAgentName(event: Event): void { emit('update:agentName', (event.target as HTMLSelectElement).value); emit('update:agentConfig', {}); }
function updateAgentMode(modeId: string): void { emit('update:agentConfig', setAgentModeValue(props.agentConfig, modeId) ?? {}); }
function updateAgentModel(event: Event): void { emit('update:agentConfig', setAgentModelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {}); }
function updateAgentThoughtLevel(event: Event): void { emit('update:agentConfig', setAgentThoughtLevelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {}); }
function updateAgentConfigOption(option: ReturnType<typeof getEditableAgentConfigOptions>[number], event: Event): void { emit('update:agentConfig', setAgentConfigOptionValue(props.agentConfig, option, (event.target as HTMLSelectElement).value) ?? {}); }
function focusDelta(delta: number): void { if (visibleTools.value.length === 0) return; const currentIndex = visibleTools.value.findIndex((tool) => tool.namespacedName === nodePaletteStore.selectedToolName); const nextIndex = currentIndex === -1 ? (delta > 0 ? 0 : visibleTools.value.length - 1) : (currentIndex + delta + visibleTools.value.length) % visibleTools.value.length; selectTool(visibleTools.value[nextIndex]!.namespacedName); }
function submitSelectedTool(): void { if (props.canSubmit) emit('submitRequest'); }
function handlePaletteKeydown(event: KeyboardEvent): void { switch (event.key) { case 'ArrowDown': event.preventDefault(); focusDelta(1); break; case 'ArrowUp': event.preventDefault(); focusDelta(-1); break; case 'Enter': case ' ': if (visibleTools.value.length === 0) return; event.preventDefault(); if (!nodePaletteStore.selectedToolName) { selectTool(visibleTools.value[0]!.namespacedName); return; } if (nodePaletteStore.selectedToolName !== props.toolName) { selectTool(nodePaletteStore.selectedToolName); return; } submitSelectedTool(); break; case 'Escape': if (!searchQuery.value) return; event.preventDefault(); clearSearch(); break; default: break; } }
function handleSearchKeydown(event: KeyboardEvent): void { if (event.key !== 'Escape' || !searchQuery.value) return; event.preventDefault(); clearSearch(); }
function highlightText(text: string, query: string): HighlightPart[] { const normalizedQuery = query.trim().toLowerCase(); if (!normalizedQuery) return [{ text, match: false }]; const normalizedText = text.toLowerCase(); const parts: HighlightPart[] = []; let cursor = 0; while (cursor < text.length) { const matchIndex = normalizedText.indexOf(normalizedQuery, cursor); if (matchIndex === -1) { parts.push({ text: text.slice(cursor), match: false }); break; } if (matchIndex > cursor) parts.push({ text: text.slice(cursor, matchIndex), match: false }); parts.push({ text: text.slice(matchIndex, matchIndex + normalizedQuery.length), match: true }); cursor = matchIndex + normalizedQuery.length; } return parts.length > 0 ? parts : [{ text, match: false }]; }

watch(() => props.active, (active) => { if (active && !hasCatalog.value) void refreshCatalogSafe(); }, { immediate: true });
watch(catalogTools, (tools) => { if (!hasLoadedCatalog.value) return; nodePaletteStore.pruneUnavailableTools(tools.map((tool) => tool.namespacedName)); if (!props.toolName && nodePaletteStore.selectedToolName) emit('update:toolName', nodePaletteStore.selectedToolName); }, { immediate: true });
watch(() => props.toolName, (value) => { nodePaletteStore.setSelectedTool(value || null); }, { immediate: true });
watch(visibleTools, (tools) => { if (!tools.length) return; if (!nodePaletteStore.selectedToolName) { selectTool(tools[0]!.namespacedName); return; } if (!tools.some((tool) => tool.namespacedName === nodePaletteStore.selectedToolName) && isSearching.value) selectTool(tools[0]!.namespacedName); }, { immediate: true });
</script>

<template>
  <div class="tool-palette field-wide">
    <ToolNodeFieldsPrompt
      :agent-name="agentName"
      :agent-config="agentConfig"
      :agent-names="configStore.agentNames"
      :selected-agent-probe="selectedAgentProbeResult"
      :ready-probe="readyProbeResult"
      :editable-agent-config-options="editableAgentConfigOptions"
      :format-agent-option-label="formatAgentOptionLabel"
      :is-probe-loading="isProbeLoading"
      :get-agent-mode-value="getAgentModeValue"
      :get-agent-model-value="getAgentModelValue"
      :get-agent-thought-level-value="getAgentThoughtLevelValue"
      :get-agent-config-option-value="getAgentConfigOptionValue"
      :is-session-config-select-group="isSessionConfigSelectGroup"
      :update-agent-name="updateAgentName"
      :update-agent-mode="updateAgentMode"
      :update-agent-model="updateAgentModel"
      :update-agent-thought-level="updateAgentThoughtLevel"
      :update-agent-config-option="updateAgentConfigOption"
    />
    <ToolNodeFieldsThinking
      :tool-name="toolName"
      :loading="loading"
      :error="error"
      :has-catalog="hasCatalog"
      :search-query="searchQuery"
      :is-searching="isSearching"
      :search-results="searchResults"
      :visible-tools="visibleTools"
      :active-descendant-id="activeDescendantId"
      :selected-tool-name="nodePaletteStore.selectedToolName"
      :recent-tools="nodePaletteStore.recentTools"
      :is-tool-pinned="(name: string) => nodePaletteStore.isToolPinned(name)"
      :option-id="optionId"
      :highlight-text="highlightText"
      :refresh-catalog="refreshCatalogSafe"
      :clear-search="clearSearch"
      :handle-search-keydown="handleSearchKeydown"
      :handle-palette-keydown="handlePaletteKeydown"
      :select-tool="selectTool"
      :update-tool-name="updateToolName"
      @update:searchQuery="searchQuery = $event"
    />
    <ToolNodeFieldsWorkflow
      :tool-name="toolName"
      :tool-args="toolArgs"
      :can-submit="canSubmit"
      :selected-tool-display-name="selectedToolDisplayName"
      :selected-tool-description="selectedToolDescription"
      :selected-tool-server="selectedToolServer"
      :required-arg-names="requiredArgNames"
      :selected-tool-schema="selectedToolSchema as Record<string, unknown> | null | undefined"
      :is-tool-pinned="(name: string) => nodePaletteStore.isToolPinned(name)"
      :toggle-pin="(name: string) => nodePaletteStore.togglePinnedTool(name)"
      :clear-selection="clearSelection"
      :on-tool-args-change="onToolArgsChange"
      :on-tool-args-validation="onToolArgsValidation"
      :submit-selected-tool="submitSelectedTool"
    />
  </div>
</template>
