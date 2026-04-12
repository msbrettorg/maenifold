import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import {
  createNode,
  type LogEntry,
  type PipelineDefinition,
  type PipelineEdge,
  type PipelineInputs,
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
import { type PipelineTemplate } from '../lib/pipeline-templates';
import { instantiateTemplate, resolveAgents } from '../lib/pipeline-template-runtime';
import { createNodeId } from '../lib/pipeline-node-factory';
import { createPipelineLifecycleActions } from './pipeline-lifecycle-actions';
import { createPipelineMutationActions } from './pipeline-mutation-actions';
import { useConfigStore } from './config';
import { useToastStore } from './toast';
import {
  isCreateResult,
  normalizePipelineList,
  normalizeView,
  serializePipelineDefinition,
  toLocalDefinition,
} from './pipeline-view-transforms';
import { hydrateEventLogFromView } from '../composables/pipeline-view-diff';

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
  const configStore = useConfigStore();
  const toastStore = useToastStore();
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

  function clearError() {
    errorMessage.value = null;
  }
  function clearSuccess() {
    successMessage.value = null;
  }
  function setSuccessMessage(message: string | null) {
    successMessage.value = message;
  }
  function setError(tool: string, err: unknown) {
    const message = `${tool}: ${err instanceof Error ? err.message : String(err)}`;
    errorMessage.value = message;
    successMessage.value = null;
    toastStore.addToast({ message, level: 'error' });
  }
  async function withError<T>(tool: string, fn: () => Promise<T>): Promise<T | undefined> {
    clearError();
    return withLoading(async () => {
      try { return await fn(); }
      catch (err) { setError(tool, err); return undefined; }
    });
  }
  const designMode = ref(true);
  const hasDirtyDraft = computed(() => designMode.value && (localDefinition.value?.nodes?.length ?? 0) > 0);
  const eventLog = ref<LogEntry[]>([]);
  const workflows = ref<string[]>([]);
  const tools = ref<ToolDefinition[]>([]);
  const runInputDrafts = ref<Record<string, PipelineInputs>>({});
  let stopPolling: (() => void) | null = null;
  const isRunning = computed(() => currentView.value?.running === true);
  const pipelineStatus = computed(() => currentView.value?.status ?? null);
  const primaryAgent = computed(() => resolveAgents(configStore.agentNames).primaryAgent.trim());
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
  function seedRunInputDraftFromView(view: PipelineView | null) {
    if (!view?.pipelineId) {
      return;
    }

    const existingDraft = runInputDrafts.value[view.pipelineId];
    if (existingDraft && Object.keys(existingDraft).length > 0) {
      return;
    }

    if (!view.inputs || Object.keys(view.inputs).length === 0) {
      return;
    }

    runInputDrafts.value = {
      ...runInputDrafts.value,
      [view.pipelineId]: { ...view.inputs },
    };
  }
  function setView(
    view: PipelineView | null,
    metadata?: {
      templateId?: string;
    },
  ) {
    currentView.value = view;
    seedRunInputDraftFromView(currentView.value);
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
  async function loadPipeline(pipelineId: string): Promise<boolean> {
    const view = await withError('view_pipeline', async () => normalizeView(await mcp.viewPipeline(pipelineId)));
    if (!view) {
      return false;
    }

    stopCurrentPolling();
    currentPipelineId.value = pipelineId;
    setView(view);
    designMode.value = false;
    eventLog.value = hydrateEventLogFromView(view);
    if (!TERMINAL_PIPELINE_STATUSES.has(view.status)) {
      startViewPolling();
    }

    return true;
  }
  function newPipeline() {
    stopCurrentPolling();
    currentPipelineId.value = null;
    setView(null);
    maxConcurrency.value = 8;
    designMode.value = true;
    eventLog.value = [];
  }
  async function createPipeline(definitionOverride?: PipelineDefinition) {
    return withError('create_pipeline', async () => {
      const definition = serializePipelineDefinition(definitionOverride ?? localDefinition.value);
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
          startViewPolling();
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
  async function applyTemplate(template: PipelineTemplate): Promise<boolean> {
    const agents = resolveAgents(configStore.agentNames);
    const agent = agents.primaryAgent.trim();
    if (agent.length === 0) {
      setError('apply_template', new Error('No agent is available for template-based creation. Configure an agent before creating a pipeline.'));
      return false;
    }

    const result = instantiateTemplate(template, agents);
    const definition = withLocalMetadata(result.definition, {
      templateId: template.id,
    });
    localDefinition.value = definition;
    return true;
  }
  function buildIntentDefinition(prompt: string, agent: string): PipelineDefinition {
    const nodeId = createNodeId('prompt');

    return {
      nodes: [
        createNode('prompt', {
          id: nodeId,
          prompt,
          agent,
          displayLabel: 'Intent',
        }),
      ],
      edges: [],
    };
  }
  async function createIntentPipeline(intent: string): Promise<boolean> {
    const prompt = intent.trim();
    if (prompt.length === 0) {
      return false;
    }

    const agent = primaryAgent.value;
    if (agent.length === 0) {
      setError('create_intent_pipeline', new Error('No agent is available for intent-based creation. Configure an agent before creating a pipeline.'));
      return false;
    }

    const definition = buildIntentDefinition(prompt, agent);
    localDefinition.value = definition;
    return true;
  }
  function getRunInputDraft(pipelineSessionId: string | null | undefined): PipelineInputs {
    if (!pipelineSessionId) {
      return {};
    }

    return { ...(runInputDrafts.value[pipelineSessionId] ?? {}) };
  }
  function setRunInputDraft(pipelineSessionId: string, key: string, value: string) {
    runInputDrafts.value = {
      ...runInputDrafts.value,
      [pipelineSessionId]: {
        ...(runInputDrafts.value[pipelineSessionId] ?? {}),
        [key]: value,
      },
    };
  }
  function replaceRunInputDraft(pipelineSessionId: string, inputs: PipelineInputs) {
    runInputDrafts.value = {
      ...runInputDrafts.value,
      [pipelineSessionId]: { ...inputs },
    };
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
  async function refreshView(options?: { resetEventLog?: boolean }) {
    if (!currentPipelineId.value) return;
    await withLoading(async () => {
      const previousView = currentView.value;
      const nextView = normalizeView(await mcp.viewPipeline(currentPipelineId.value!));
      if (options?.resetEventLog) {
        eventLog.value = hydrateEventLogFromView(nextView);
      } else if (previousView) {
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
    refreshView,
    startViewPolling,
  };
});
