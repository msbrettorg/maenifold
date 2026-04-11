import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import {
  createNode,
  type LogEntry,
  type PipelineDefinition,
  type PipelineEdge,
  type PipelineListItem,
  type PipelineNode,
  type PipelineNodeType,
  type PipelineView,
} from '../lib/pipeline-types';
import type {
  PipelineNodeInput,
  PromptNodeInput,
  SequentialThinkingNodeInput,
  ToolNodeInput,
  WorkflowNodeInput,
} from '../lib/pipeline-node-factory';
import { type PipelineTemplate, type TemplateVariables } from '../lib/pipeline-templates';
import { instantiateTemplate } from '../lib/pipeline-template-runtime';
import { createPipelineLifecycleActions } from './pipeline-lifecycle-actions';
import { createPipelineMutationActions } from './pipeline-mutation-actions';
import {
  isCreateResult,
  normalizePipelineList,
  normalizeView,
  serializePipelineDefinition,
  toLocalDefinition,
} from './pipeline-view-transforms';

const EMPTY_DEFINITION = (): PipelineDefinition => ({ nodes: [], edges: [] });
const TERMINAL_PIPELINE_STATUSES: ReadonlySet<PipelineView['status']> = new Set(['completed', 'cancelled', 'stuck']);

interface ToolDefinition {
  name: string;
  description?: string;
  inputSchema?: unknown;
}

function serializeEdge(edge: PipelineEdge): PipelineEdge {
  return {
    source: edge.source,
    target: edge.target,
    ...(edge.type ? { type: edge.type } : {}),
    ...(edge.mapping && Object.keys(edge.mapping).length > 0
      ? { mapping: { ...edge.mapping } }
      : {}),
  };
}

export const usePipelineStore = defineStore('pipeline', () => {
  const mcp = usePipelineMcp();
  const pipelines = ref<PipelineListItem[]>([]);
  const currentPipelineId = ref<string | null>(null);
  const currentView = ref<PipelineView | null>(null);
  const localDefinition = ref<PipelineDefinition>(EMPTY_DEFINITION());
  const maxConcurrency = ref(8);
  const pendingRequests = ref(0);
  const loading = computed(() => pendingRequests.value > 0);
  const errorMessage = ref<string | null>(null);
  async function withLoading<T>(fn: () => Promise<T>): Promise<T> {
    pendingRequests.value += 1;
    try {
      return await fn();
    } finally {
      pendingRequests.value = Math.max(0, pendingRequests.value - 1);
    }
  }

  function clearError() {
    errorMessage.value = null;
  }
  function setError(tool: string, err: unknown) {
    errorMessage.value = `${tool}: ${err instanceof Error ? err.message : String(err)}`;
  }
  async function withError<T>(tool: string, fn: () => Promise<T>): Promise<T | undefined> {
    clearError();
    return withLoading(async () => {
      try { return await fn(); }
      catch (err) { setError(tool, err); return undefined; }
    });
  }
  const designMode = ref(true);
  const eventLog = ref<LogEntry[]>([]);
  const workflows = ref<string[]>([]);
  const tools = ref<ToolDefinition[]>([]);
  let stopPolling: (() => void) | null = null;
  const isRunning = computed(() => currentView.value?.running === true);
  const pipelineStatus = computed(() => currentView.value?.status ?? null);
  function withLocalMetadata(
    definition: PipelineDefinition,
    metadata?: {
      templateId?: string;
    },
  ): PipelineDefinition {
    const templateId = metadata?.templateId;
    return {
      ...definition,
      ...(templateId ? { templateId } : {}),
    };
  }
  function setView(
    view: PipelineView | null,
    metadata?: {
      templateId?: string;
    },
  ) {
    currentView.value = view;
    localDefinition.value = view
      ? withLocalMetadata(toLocalDefinition(view), metadata)
      : EMPTY_DEFINITION();
    if (view && !view.running && TERMINAL_PIPELINE_STATUSES.has(view.status)) stopCurrentPolling();
  }
  function appendNode(node: PipelineNode): PipelineNode {
    localDefinition.value = {
      ...localDefinition.value,
      nodes: [...localDefinition.value.nodes, node],
    };
    return node;
  }
  async function connect() {
    await withLoading(async () => {
      await mcp.connect();
      if (!mcp.connected.value) {
        throw new Error(mcp.error.value ?? 'Failed to connect to MCP service');
      }
    });
  }
  function disconnect() {
    stopCurrentPolling();
    mcp.disconnect();
  }
  async function refreshList() {
    await withLoading(async () => {
      pipelines.value = normalizePipelineList(await mcp.listPipelines());
    });
  }
  async function loadPipeline(pipelineId: string) {
    await withError('view_pipeline', async () => {
      stopCurrentPolling();
      currentPipelineId.value = pipelineId;
      setView(normalizeView(await mcp.viewPipeline(pipelineId)));
      designMode.value = false;
      eventLog.value = [];
    });
  }
  function newPipeline() {
    stopCurrentPolling();
    currentPipelineId.value = null;
    setView(null);
    maxConcurrency.value = 8;
    designMode.value = true;
    eventLog.value = [];
  }
  async function createPipeline() {
    return withError('create_pipeline', async () => {
      const definition = serializePipelineDefinition(localDefinition.value);
      definition.nodes.forEach((node) => {
        if (node.type === 'workflow' && !('workflowId' in node && node.workflowId)) {
          throw new Error(`Workflow node '${node.id}' is missing workflowId`);
        }
      });
      const result = await mcp.createPipeline({
        ...definition,
        edges: definition.edges.map(serializeEdge),
      }, maxConcurrency.value);
      if (isCreateResult(result) && typeof result.pipelineId === 'string' && result.pipelineId.length > 0) {
        const pipelineId = result.pipelineId;
        currentPipelineId.value = pipelineId;
        designMode.value = false;
        await withError('view_pipeline', async () => {
          stopCurrentPolling();
          currentPipelineId.value = pipelineId;
          setView(normalizeView(await mcp.viewPipeline(pipelineId)));
          designMode.value = false;
          eventLog.value = [];
        });
        await refreshList();
      }
      return result;
    });
  }
  function addNode(node: PipelineNode): PipelineNode;
  function addNode(type: 'workflow', fields: WorkflowNodeInput): PipelineNode;
  function addNode(type: 'prompt', fields: PromptNodeInput): PipelineNode;
  function addNode(type: 'sequentialThinking', fields: SequentialThinkingNodeInput): PipelineNode;
  function addNode(type: 'tool', fields: ToolNodeInput): PipelineNode;
  function addNode(nodeOrType: PipelineNode | PipelineNodeType, fields?: PipelineNodeInput): PipelineNode {
    if (typeof nodeOrType !== 'string') {
      return appendNode(nodeOrType);
    }

    if (!fields) {
      throw new Error(`Missing fields for ${nodeOrType} node`);
    }

    return appendNode(createNode(nodeOrType, fields));
  }
  function applyTemplate(template: PipelineTemplate, variables: TemplateVariables) {
    const result = instantiateTemplate(template, variables);
    localDefinition.value = withLocalMetadata(result.definition, {
      templateId: template.id,
    });
  }
  async function refreshWorkflows() {
    await withLoading(async () => {
      try { workflows.value = await mcp.listWorkflows(); } catch { /* non-critical */ }
    });
  }
  function listWorkflowSummaries() { return mcp.listWorkflowSummaries(); }
  function readWorkflowDefinition(id: string) { return mcp.readWorkflowDefinition(id); }
  async function refreshTools() {
    await withLoading(async () => {
      try { tools.value = await mcp.listTools(); } catch { /* non-critical */ }
    });
  }
  async function refreshView() {
    if (!currentPipelineId.value) return;
    await withLoading(async () => {
      const previousView = currentView.value;
      const nextView = normalizeView(await mcp.viewPipeline(currentPipelineId.value!));
      if (previousView) {
        const entries = mcp.diffViews(previousView, nextView);
        if (entries.length > 0) {
          eventLog.value = [...eventLog.value, ...entries];
        }
      }
      setView(nextView);
    });
  }
  function startViewPolling() {
    stopCurrentPolling();
    if (!currentPipelineId.value) return;
    stopPolling = mcp.startPolling(currentPipelineId.value, (view) => {
      const nextView = normalizeView(view);
      if (currentView.value) {
        const entries = mcp.diffViews(currentView.value, nextView);
        if (entries.length > 0) {
          eventLog.value = [...eventLog.value, ...entries];
        }
      }
      setView(nextView);
    }, 3000);
  }
  function stopCurrentPolling() {
    if (!stopPolling) return;
    stopPolling();
    stopPolling = null;
  }
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
  });
  const { forkPipeline, joinPipeline, dispatchPipeline } = createPipelineMutationActions({ mcp, currentPipelineId, refreshView, refreshList, startViewPolling, withError });
  return {
    pipelines,
    currentPipelineId,
    currentView,
    localDefinition,
    maxConcurrency,
    loading,
    designMode,
    eventLog,
    workflows,
    tools,
    isRunning,
    pipelineStatus,
    connected: mcp.connected,
    error: mcp.error,
    errorMessage,
    clearError,
    connect,
    disconnect,
    refreshList,
    loadPipeline,
    newPipeline,
    createPipeline,
    addNode,
    applyTemplate,
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
    refreshView,
    startViewPolling,
  };
});
