import { computed, ref, watch } from 'vue';
import type { AgentProbeResult } from '../lib/agent-probe-types';
import { useMcpToolDiscovery } from './useMcpToolDiscovery';
import { usePipelineMcp } from './usePipelineMcp';
import { useReadyAgentProbe } from './useReadyAgentProbe';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import {
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
} from './pipeline-node-form-helpers';
import { deriveServerHealth, getAuthSetupAction } from '../lib/palette-health';
import {
  buildToolPaletteEntries,
  categorizeTools,
  getPaletteEntrySearchFields,
  scoredSearch,
  type ToolPaletteEntry,
} from '../lib/tool-palette-catalog';
import { writeLastUsedAgent } from '../lib/preferred-agent';
import { resolveToolDefinition } from '../lib/tool-categories';
import type { ServerHealthStatus } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { useNodePaletteStore } from '../stores/node-palette';
import { usePipelineStore } from '../stores/pipeline';

interface ToolPaletteProps {
  active?: boolean;
  toolName: string;
  agentName?: string;
  agentConfig?: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  canSubmit?: boolean;
}

type ToolPaletteEmit = {
  (event: 'update:toolName', value: string): void;
  (event: 'update:agentName', value: string): void;
  (event: 'update:agentConfig', value: PipelineNodeAgentConfig): void;
  (event: 'update:toolArgs', value: Record<string, unknown>): void;
  (event: 'validation', value: { valid: boolean }): void;
  (event: 'submitRequest'): void;
};

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isToolEntry(entry: ToolPaletteEntry): entry is ToolPaletteEntry & { kind: 'tool'; toolName: string; inputSchema: Record<string, unknown> } {
  return entry.kind === 'tool' && entry.toolName !== null && entry.inputSchema !== null;
}

export function useToolPaletteFields(props: ToolPaletteProps, emit: ToolPaletteEmit) {
  const mcp = usePipelineMcp();
  const pipelineStore = usePipelineStore();
  const configStore = useConfigStore();
  const capabilitiesStore = useAgentCapabilitiesStore();
  const nodePaletteStore = useNodePaletteStore();
  const { serverResults, loading, error, hasLoadedCatalog, refreshTools, hasStaleCacheByServer } = useMcpToolDiscovery();
  const activeOptionName = ref<string | null>(props.toolName || null);
  const schemaValid = ref(true);
  const authDialogTool = ref<ToolPaletteEntry | null>(null);
  const authDialogError = ref<string | null>(null);
  const authDialogBusy = ref(false);

  const catalogTools = computed<ToolPaletteEntry[]>(() =>
    buildToolPaletteEntries(serverResults.value, hasStaleCacheByServer.value),
  );
  const hasCatalog = computed(() => catalogTools.value.length > 0);
  const searchQuery = computed({
    get: () => nodePaletteStore.searchQuery,
    set: (value: string) => nodePaletteStore.setSearchQuery(value),
  });
  const isSearching = computed(() => searchQuery.value.trim().length > 0);
  const selectedDiscoveryTool = computed(() =>
    catalogTools.value.find((tool) => isToolEntry(tool) && tool.toolName === props.toolName) ?? null,
  );
  const selectedFallbackTool = computed(() => resolveToolDefinition(pipelineStore.tools, props.toolName) ?? null);
  const selectedToolSchema = computed(
    () => selectedDiscoveryTool.value?.inputSchema ?? selectedFallbackTool.value?.inputSchema ?? null,
  );
  const selectedToolDescription = computed(
    () => selectedDiscoveryTool.value?.description ?? selectedFallbackTool.value?.description ?? null,
  );
  const selectedToolHealth = computed<ServerHealthStatus | null>(() => selectedDiscoveryTool.value?.health ?? null);
  const selectedToolAuthAction = computed(() =>
    selectedDiscoveryTool.value ? getAuthSetupAction(selectedDiscoveryTool.value.server) : null,
  );
  const requiredArgNames = computed(() => {
    const schema = selectedToolSchema.value;
    if (!schema || typeof schema !== 'object' || Array.isArray(schema)) return [];
    const required = (schema as { required?: unknown }).required;
    return Array.isArray(required) ? required.filter((entry): entry is string => typeof entry === 'string') : [];
  });
  const quickAddTools = computed<ToolPaletteEntry[]>(() => {
    const orderedNames = [
      ...nodePaletteStore.pinnedTools,
      ...nodePaletteStore.recentTools.filter((name) => !nodePaletteStore.pinnedTools.includes(name)),
    ];
    return orderedNames
      .map((name) => catalogTools.value.find((tool) => isToolEntry(tool) && tool.toolName === name) ?? null)
      .filter((tool): tool is ToolPaletteEntry => tool !== null);
  });
  const categorizedTools = computed(() =>
    categorizeTools(catalogTools.value, (tool) => ({
      name: tool.toolName,
      server: tool.server,
      description: tool.description,
    })),
  );
  const categorizedBrowseTools = computed<ToolPaletteEntry[]>(() =>
    categorizedTools.value.flatMap((entry) => entry.tools),
  );
  const showQuickAdd = computed(() => !isSearching.value && quickAddTools.value.length > 0);
  const showCategories = computed(() => !isSearching.value && hasCatalog.value);
  const searchResults = computed<ToolPaletteEntry[]>(() => {
    const query = searchQuery.value.trim();
    if (!query) return [];
    return scoredSearch(
      catalogTools.value,
      query,
      getPaletteEntrySearchFields,
    ).map((result) => result.item);
  });
  const visibleTools = computed(() => {
    if (isSearching.value) return searchResults.value;
    return showCategories.value ? categorizedBrowseTools.value : [];
  });
  const activeDescendantId = computed(() =>
    activeOptionName.value && visibleTools.value.some((tool) => tool.namespacedName === activeOptionName.value)
      ? optionId(activeOptionName.value)
      : undefined,
  );
  const selectionValid = computed(() => selectedToolHealth.value !== 'auth_required' && schemaValid.value);
  const selectedAgentProbeResult = computed(() =>
    props.agentName ? getProbe(props.agentName) : undefined,
  );
  const readyAgentProbe = useReadyAgentProbe(
    selectedAgentProbeResult,
    computed(() => props.agentConfig),
    (nextAgentConfig) => emit('update:agentConfig', nextAgentConfig),
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
    return selectedAgentProbeResult.value;
  }
  function readyProbe(): AgentProbeResult | undefined {
    return readyAgentProbe.value;
  }
  function focusTool(name: string | null): void {
    activeOptionName.value = name;
  }
  function commitTool(tool: ToolPaletteEntry & { kind: 'tool'; toolName: string }): void {
    nodePaletteStore.setSelectedTool(tool.toolName);
    focusTool(tool.namespacedName);
    emit('update:toolName', tool.toolName);
  }
  function clearSelection(): void {
    nodePaletteStore.setSelectedTool(null);
    emit('update:toolName', '');
    emit('update:toolArgs', {});
    schemaValid.value = true;
  }
  function clearSearch(): void {
    nodePaletteStore.clearSearchQuery();
  }
  async function refreshCatalog(): Promise<void> {
    await refreshTools();
    if (pipelineStore.tools.length === 0) {
      await pipelineStore.refreshTools();
    }
  }
  function openAuthDialog(tool: ToolPaletteEntry): void {
    authDialogTool.value = tool;
    authDialogError.value = null;
    focusTool(tool.namespacedName);
  }
  function closeAuthDialog(): void {
    authDialogTool.value = null;
    authDialogError.value = null;
    authDialogBusy.value = false;
  }
  function handleToolSelect(tool: ToolPaletteEntry): void {
    focusTool(tool.namespacedName);
    const action = tool.health === 'auth_required' ? getAuthSetupAction(tool.server) : null;
    if (action) {
      openAuthDialog(tool);
      return;
    }
    if (isToolEntry(tool)) {
      commitTool(tool);
    }
  }
  async function runAuthSetup(): Promise<void> {
    const tool = authDialogTool.value;
    const action = tool ? getAuthSetupAction(tool.server) : null;
    if (!tool || !action) return;
    authDialogBusy.value = true;
    authDialogError.value = null;
    try {
      if (!mcp.connected.value) await mcp.connect();
      await mcp.callTool(action.toolName, action.toolArgs);
      const refreshed = await refreshTools();
      if (pipelineStore.tools.length === 0) await pipelineStore.refreshTools();
      const next = refreshed.find((result) => result.server === tool.server);
      const nextHealth = next
        ? deriveServerHealth(next, hasStaleCacheByServer.value[next.server] ?? false)
        : null;
      if (nextHealth === 'ready') {
        if (isToolEntry(tool)) {
          commitTool({ ...tool, health: 'ready', status: 'ok', error: null });
        }
        closeAuthDialog();
        return;
      }
      authDialogError.value = `Setup completed, but ${tool.server} still reports ${nextHealth ?? 'an error'}.`;
    } catch (error) {
      authDialogError.value = getErrorMessage(error);
    } finally {
      authDialogBusy.value = false;
    }
  }
  function updateToolName(event: Event): void {
    emit('update:toolName', (event.target as HTMLInputElement).value);
  }
  function updateAgentName(event: Event): void {
    const nextAgent = (event.target as HTMLSelectElement).value;
    emit('update:agentName', nextAgent);
    emit('update:agentConfig', {});
    writeLastUsedAgent(nextAgent);
  }
  function updateAgentMode(modeId: string): void {
    emit('update:agentConfig', setAgentModeValue(props.agentConfig, modeId) ?? {});
  }
  function updateAgentModel(event: Event): void {
    emit('update:agentConfig', setAgentModelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {});
  }
  function updateAgentThoughtLevel(event: Event): void {
    emit('update:agentConfig', setAgentThoughtLevelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {});
  }
  function updateAgentConfigOption(option: ReturnType<typeof getEditableAgentConfigOptions>[number], event: Event): void {
    emit('update:agentConfig', setAgentConfigOptionValue(props.agentConfig, option, (event.target as HTMLSelectElement).value) ?? {});
  }
  function onToolArgsChange(data: Record<string, unknown>): void {
    emit('update:toolArgs', data);
  }
  function onToolArgsValidation(state: { valid: boolean }): void {
    schemaValid.value = state.valid;
  }
  function submitSelectedTool(): void {
    if (selectedToolHealth.value === 'auth_required' && selectedToolAuthAction.value && selectedDiscoveryTool.value) {
      openAuthDialog(selectedDiscoveryTool.value);
      return;
    }
    if (props.canSubmit) emit('submitRequest');
  }
  function handlePaletteKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && searchQuery.value) {
      event.preventDefault();
      clearSearch();
      return;
    }
    if (visibleTools.value.length === 0 || !['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const currentIndex = visibleTools.value.findIndex((tool) => tool.namespacedName === activeOptionName.value);
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      const nextIndex = currentIndex === -1
        ? (delta > 0 ? 0 : visibleTools.value.length - 1)
        : (currentIndex + delta + visibleTools.value.length) % visibleTools.value.length;
      focusTool(visibleTools.value[nextIndex]!.namespacedName);
      return;
    }
    event.preventDefault();
    const activeTool = visibleTools.value.find((tool) => tool.namespacedName === activeOptionName.value) ?? visibleTools.value[0]!;
    if (activeTool.namespacedName !== props.toolName || activeTool.health === 'auth_required') {
      handleToolSelect(activeTool);
      return;
    }
    submitSelectedTool();
  }
  function highlightText(text: string, query: string) {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) return [{ text, match: false }];
    const normalizedText = text.toLowerCase();
    const parts: Array<{ text: string; match: boolean }> = [];
    let cursor = 0;
    while (cursor < text.length) {
      const matchIndex = normalizedText.indexOf(normalizedQuery, cursor);
      if (matchIndex === -1) return [...parts, { text: text.slice(cursor), match: false }];
      if (matchIndex > cursor) parts.push({ text: text.slice(cursor, matchIndex), match: false });
      parts.push({ text: text.slice(matchIndex, matchIndex + normalizedQuery.length), match: true });
      cursor = matchIndex + normalizedQuery.length;
    }
    return parts.length > 0 ? parts : [{ text, match: false }];
  }

  watch(() => props.active, (active) => { if (active && !hasCatalog.value) void refreshCatalog(); }, { immediate: true });
  watch(catalogTools, (tools) => {
    if (!hasLoadedCatalog.value) return;
    nodePaletteStore.pruneUnavailableTools(
      tools.filter(isToolEntry).map((tool) => tool.toolName),
    );
    if (!props.toolName && nodePaletteStore.selectedToolName) emit('update:toolName', nodePaletteStore.selectedToolName);
  }, { immediate: true });
  watch(() => props.toolName, (value) => {
    nodePaletteStore.setSelectedTool(value || null);
    schemaValid.value = true;
    if (value) focusTool(value);
  }, { immediate: true });
  watch(visibleTools, (tools) => {
    if (!tools.length) return;
    if (!activeOptionName.value || !tools.some((tool) => tool.namespacedName === activeOptionName.value)) {
      focusTool(tools[0]!.namespacedName);
    }
  }, { immediate: true });
  watch(selectionValid, (valid) => emit('validation', { valid }), { immediate: true });

  return {
    activeDescendantId,
    authDialogAction: computed(() => (authDialogTool.value ? getAuthSetupAction(authDialogTool.value.server) : null)),
    authDialogBusy,
    authDialogError,
    authDialogTool,
    categorizedTools,
    catalogTools,
    clearSearch,
    clearSelection,
    closeAuthDialog,
    configStore,
    error,
    formatAgentOptionLabel,
    getAgentConfigOptionValue,
    getAgentModeValue,
    getAgentModelValue,
    getAgentThoughtLevelValue,
    getEditableAgentConfigOptions,
    handlePaletteKeydown,
    handleToolSelect,
    hasCatalog,
    hasLoadedCatalog,
    highlightText,
    isProbeLoading: (agentName: string) => capabilitiesStore.probeStatus === 'probing' && !getProbe(agentName),
    isSearching,
    isSessionConfigSelectGroup,
    loading,
    nodePaletteStore,
    onToolArgsChange,
    onToolArgsValidation,
    optionId,
    quickAddTools,
    readyProbe,
    refreshCatalog,
    requiredArgNames,
    runAuthSetup,
    searchQuery,
    searchResults,
    selectedAgentProbe,
    selectedDiscoveryTool,
    selectedToolAuthAction,
    selectedToolDescription,
    selectedToolHealth,
    selectedToolSchema,
    showCategories,
    showQuickAdd,
    submitSelectedTool,
    updateAgentConfigOption,
    updateAgentMode,
    updateAgentModel,
    updateAgentName,
    updateAgentThoughtLevel,
    updateToolName,
    visibleTools,
  };
}
