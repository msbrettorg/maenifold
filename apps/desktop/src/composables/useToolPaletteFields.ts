import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import type { AgentProbeResult } from '../lib/agent-probe-types-defs';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import { usePipelineMcp } from './usePipelineMcp';
import { useReadyAgentProbe } from './useReadyAgentProbe';
import {
  formatPipelineAgentOptionLabel,
  getAgentConfigOptionValue,
  getAgentModelValue,
  getAgentThoughtLevelValue,
  getEditableAgentConfigOptions,
  getAgentModeValue,
  isSessionConfigSelectGroup,
} from './pipeline-node-form-helpers';
import { deriveServerHealth, getAuthSetupAction } from '../lib/palette-health';
import {
  buildToolPaletteEntries,
  categorizeTools,
  getPaletteEntrySearchFields,
  scoredSearch,
  type ToolPaletteEntry,
} from '../lib/tool-palette-catalog';
import { resolveToolDefinition } from '../lib/tool-categories';
import type { ServerHealthStatus } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { useNodePaletteStore } from '../stores/node-palette';
import { usePipelineStore } from '../stores/pipeline';
import {
  createAgentUpdaters,
  createPaletteKeydownHandler,
  getErrorMessage,
  highlightText,
  isToolEntry,
  optionId,
  type ToolPaletteEmit,
  type ToolPaletteProps,
} from './useToolPaletteFields-helpers';

export type { ToolPaletteEmit, ToolPaletteProps } from './useToolPaletteFields-helpers';

export function useToolPaletteFields(props: ToolPaletteProps, emit: ToolPaletteEmit) {
  const mcp = usePipelineMcp();
  const pipelineStore = usePipelineStore();
  const configStore = useConfigStore();
  const capabilitiesStore = useAgentCapabilitiesStore();
  const nodePaletteStore = useNodePaletteStore();
  const mcpToolDiscoveryStore = useMcpToolDiscoveryStore();
  const { serverResults, loading, error, hasLoadedCatalog, hasStaleCacheByServer } = storeToRefs(mcpToolDiscoveryStore);
  const { refreshTools } = mcpToolDiscoveryStore;
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
    return scoredSearch(catalogTools.value, query, getPaletteEntrySearchFields).map((result) => result.item);
  });
  const visibleTools = computed(() => (isSearching.value ? searchResults.value : showCategories.value ? categorizedBrowseTools.value : []));
  const activeDescendantId = computed(() =>
    activeOptionName.value && visibleTools.value.some((tool) => tool.namespacedName === activeOptionName.value)
      ? optionId(activeOptionName.value)
      : undefined,
  );
  const selectionValid = computed(() => selectedToolHealth.value !== 'auth_required' && schemaValid.value);
  const selectedAgentProbeResult = computed(() => (props.agentName ? getProbe(props.agentName) : undefined));
  const readyAgentProbe = useReadyAgentProbe(
    selectedAgentProbeResult,
    computed(() => props.agentConfig),
    (nextAgentConfig) => emit('update:agentConfig', nextAgentConfig),
  );

  const getProbe = (agentName: string): AgentProbeResult | undefined =>
    agentName ? capabilitiesStore.getProbe(agentName) : undefined;
  const formatAgentOptionLabel = (agentName: string): string =>
    formatPipelineAgentOptionLabel(agentName, getProbe(agentName));
  const selectedAgentProbe = (): AgentProbeResult | undefined => selectedAgentProbeResult.value;
  const readyProbe = (): AgentProbeResult | undefined => readyAgentProbe.value;
  const focusTool = (name: string | null): void => { activeOptionName.value = name; };
  const commitTool = (tool: ToolPaletteEntry & { kind: 'tool'; toolName: string }): void => {
    nodePaletteStore.setSelectedTool(tool.toolName);
    focusTool(tool.namespacedName);
    emit('update:toolName', tool.toolName);
  };
  const clearSelection = (): void => {
    nodePaletteStore.setSelectedTool(null);
    emit('update:toolName', '');
    emit('update:toolArgs', {});
    schemaValid.value = true;
  };
  const clearSearch = (): void => { nodePaletteStore.clearSearchQuery(); };
  const refreshCatalog = async (): Promise<void> => {
    await refreshTools();
    if (pipelineStore.tools.length === 0) await pipelineStore.refreshTools();
  };
  const openAuthDialog = (tool: ToolPaletteEntry): void => {
    authDialogTool.value = tool;
    authDialogError.value = null;
    focusTool(tool.namespacedName);
  };
  const closeAuthDialog = (): void => {
    authDialogTool.value = null;
    authDialogError.value = null;
    authDialogBusy.value = false;
  };
  function handleToolSelect(tool: ToolPaletteEntry): void {
    focusTool(tool.namespacedName);
    const action = tool.health === 'auth_required' ? getAuthSetupAction(tool.server) : null;
    if (action) {
      openAuthDialog(tool);
      return;
    }
    if (isToolEntry(tool)) commitTool(tool);
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
      const nextHealth = next ? deriveServerHealth(next, hasStaleCacheByServer.value[next.server] ?? false) : null;
      if (nextHealth === 'ready') {
        if (isToolEntry(tool)) commitTool({ ...tool, health: 'ready', status: 'ok', error: null });
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

  const agentUpdaters = createAgentUpdaters(props, emit);
  const handlePaletteKeydown = createPaletteKeydownHandler({
    searchQuery,
    visibleTools,
    activeOptionName,
    toolName: () => props.toolName,
    clearSearch,
    focusTool,
    handleToolSelect,
    submitSelectedTool,
  });

  watch(() => props.active, (active) => { if (active && !hasCatalog.value) void refreshCatalog(); }, { immediate: true });
  watch(catalogTools, (tools) => {
    if (!hasLoadedCatalog.value) return;
    nodePaletteStore.pruneUnavailableTools(tools.filter(isToolEntry).map((tool) => tool.toolName));
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
    updateAgentConfigOption: agentUpdaters.updateAgentConfigOption,
    updateAgentMode: agentUpdaters.updateAgentMode,
    updateAgentModel: agentUpdaters.updateAgentModel,
    updateAgentName: agentUpdaters.updateAgentName,
    updateAgentThoughtLevel: agentUpdaters.updateAgentThoughtLevel,
    updateToolName,
    visibleTools,
  };
}
