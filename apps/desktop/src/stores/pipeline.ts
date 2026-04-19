import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { toast } from 'vue-sonner';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import {
  type LogEntry,
  type PipelineDefinition,
  type PipelineInputs,
  type PipelineListItem,
  type PipelineView,
} from '../lib/pipeline-types';
import { resolvePreferredAgent } from '../lib/preferred-agent';
import { createPipelineLifecycleActions } from './pipeline-lifecycle-actions';
import { createPipelineMutationActions } from './pipeline-mutation-actions';
import { useConfigStore } from './config';
import {
  normalizePipelineList,
  toLocalDefinition,
} from './pipeline-view-transforms';
import { createPipelineRunnerActions } from './pipeline-runner';
import {
  createPipelineRefreshActions,
  createRunInputDraftActions,
  seedRunInputDraftFromView,
  serializeEdge,
  withLocalMetadata,
} from './pipeline-sync';
import { createPipelineDesignActions } from './pipeline-design';

const EMPTY_DEFINITION = (): PipelineDefinition => ({ nodes: [], edges: [] });

interface ToolDefinition {
  name: string;
  description?: string;
  inputSchema?: unknown;
}

interface AssetSummaryOption {
  id: string;
}

export const usePipelineStore = defineStore('pipeline', () => {
  const mcp = usePipelineMcp();
  const configStore = useConfigStore();
  const pipelines = ref<PipelineListItem[]>([]);
  const currentPipelineId = ref<string | null>(null);
  const currentView = ref<PipelineView | null>(null);
  const localDefinition = ref<PipelineDefinition>(EMPTY_DEFINITION());
  const maxConcurrency = ref(8);
  const pendingRequests = ref(0);
  const loading = computed(() => pendingRequests.value > 0);
  const errorMessage = ref<string | null>(null);
  const successMessage = ref<string | null>(null);
  async function withLoading<T>(fn: () => Promise<T>): Promise<T> {
    pendingRequests.value += 1;
    try {
      return await fn();
    } finally {
      pendingRequests.value = Math.max(0, pendingRequests.value - 1);
    }
  }

  function clearError() { errorMessage.value = null; }
  function clearSuccess() { successMessage.value = null; }
  function setSuccessMessage(message: string | null) { successMessage.value = message; }
  function setError(tool: string, err: unknown) {
    const message = `${tool}: ${err instanceof Error ? err.message : String(err)}`;
    errorMessage.value = message;
    successMessage.value = null;
    toast.error(message);
  }
  async function withError<T>(tool: string, fn: () => Promise<T>): Promise<T | undefined> {
    clearError();
    return withLoading(async () => {
      try { return await fn(); }
      catch (err) { setError(tool, err); return undefined; }
    });
  }

  const designMode = ref(true);
  const hasDirtyDraft = computed(
    () => designMode.value && (localDefinition.value?.nodes?.length ?? 0) > 0,
  );
  const eventLog = ref<LogEntry[]>([]);
  const workflows = ref<string[]>([]);
  const tools = ref<ToolDefinition[]>([]);
  const roleAssets = ref<AssetSummaryOption[]>([]);
  const colorAssets = ref<AssetSummaryOption[]>([]);
  const perspectiveAssets = ref<AssetSummaryOption[]>([]);
  const runInputDrafts = ref<Record<string, PipelineInputs>>({});
  const isRunning = computed(() => currentView.value?.running === true);
  const pipelineStatus = computed(() => currentView.value?.status ?? null);
  const primaryAgent = computed(() => resolvePreferredAgent(configStore.agentNames).trim());

  function setView(view: PipelineView | null, metadata?: { templateId?: string }) {
    currentView.value = view;
    seedRunInputDraftFromView(runInputDrafts, currentView.value);
    localDefinition.value = view
      ? withLocalMetadata(toLocalDefinition(view), metadata)
      : EMPTY_DEFINITION();
    syncPollingState(view);
  }

  const {
    startViewPolling,
    stopCurrentPolling,
    syncPollingState,
    resumePollingForActiveView,
    refreshView,
  } = createPipelineRunnerActions({
    mcp, currentPipelineId, currentView, eventLog, setView, withLoading,
  });

  const { refreshWorkflows, refreshTools, refreshAdoptAssets } = createPipelineRefreshActions({
    mcp, workflows, tools, roleAssets, colorAssets, perspectiveAssets, withLoading,
  });

  const { getRunInputDraft, setRunInputDraft, replaceRunInputDraft } =
    createRunInputDraftActions(runInputDrafts);

  async function refreshList() {
    await withLoading(async () => {
      pipelines.value = normalizePipelineList(await mcp.listPipelines());
    });
  }

  const {
    addNode,
    loadPipeline,
    newPipeline,
    createPipeline,
    applyTemplate,
    createIntentPipeline,
    connect,
    disconnect,
  } = createPipelineDesignActions({
    mcp,
    configStore,
    currentPipelineId,
    currentView,
    localDefinition,
    pipelines,
    designMode,
    eventLog,
    maxConcurrency,
    primaryAgent,
    setView,
    stopCurrentPolling,
    refreshList,
    setError,
    withError,
    withLoading,
  });

  function listWorkflowSummaries() { return mcp.listWorkflowSummaries(); }
  function readWorkflowDefinition(id: string) { return mcp.readWorkflowDefinition(id); }

  const {
    runPipeline,
    pausePipeline,
    resumePipeline,
    cancelPipeline,
    deletePipeline,
    deletePipelineById,
    rewindNodes,
    restartNode,
    updateNodeStatus,
    graftNodes,
  } = createPipelineLifecycleActions({
    mcp,
    currentPipelineId,
    refreshView,
    refreshList,
    startViewPolling,
    stopCurrentPolling,
    newPipeline,
    withError,
    serializeEdge,
    setSuccessMessage,
  });
  const { forkPipeline, joinPipeline, dispatchPipeline } = createPipelineMutationActions({
    mcp,
    currentPipelineId,
    refreshView,
    refreshList,
    startViewPolling,
    loadPipeline,
    withError,
    setSuccessMessage,
  });
  return {
    pipelines,
    currentPipelineId,
    currentView,
    localDefinition,
    maxConcurrency,
    loading,
    designMode,
    hasDirtyDraft,
    eventLog,
    workflows,
    tools,
    roleAssets,
    colorAssets,
    perspectiveAssets,
    isRunning,
    pipelineStatus,
    connected: mcp.connected,
    error: mcp.error,
    errorMessage,
    successMessage,
    clearError,
    clearSuccess,
    setSuccessMessage,
    connect,
    disconnect,
    refreshList,
    loadPipeline,
    newPipeline,
    createPipeline,
    addNode,
    applyTemplate,
    createIntentPipeline,
    getRunInputDraft,
    setRunInputDraft,
    replaceRunInputDraft,
    runPipeline,
    pausePipeline,
    resumePipeline,
    cancelPipeline,
    deletePipeline,
    deletePipelineById,
    rewindNodes,
    forkPipeline,
    joinPipeline,
    dispatchPipeline,
    restartNode,
    updateNodeStatus,
    graftNodes,
    refreshWorkflows,
    listWorkflowSummaries,
    readWorkflowDefinition,
    refreshTools,
    refreshAdoptAssets,
    refreshView,
    resumePollingForActiveView,
    startViewPolling,
    stopPolling: stopCurrentPolling,
  };
});
