// Design-mode and creation actions extracted from pipeline.ts
// to keep the main store under the 300 LOC ceiling.
import type { Ref } from 'vue';
import type { usePipelineMcp } from '../composables/usePipelineMcp';
import type { useConfigStore } from './config';
import {
  createNode,
  type LogEntry,
  type PipelineDefinition,
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
import { resolvePreferredAgent } from '../lib/preferred-agent';
import {
  isCreateResult,
  normalizeView,
  serializePipelineDefinition,
} from './pipeline-view-transforms';
import { hydrateEventLogFromView } from '../composables/pipeline-view-diff';
import {
  buildIntentDefinition,
  serializeEdge,
  withLocalMetadata,
} from './pipeline-sync';

interface DesignActionsDeps {
  mcp: ReturnType<typeof usePipelineMcp>;
  configStore: ReturnType<typeof useConfigStore>;
  currentPipelineId: Ref<string | null>;
  currentView: Ref<PipelineView | null>;
  localDefinition: Ref<PipelineDefinition>;
  pipelines: Ref<PipelineListItem[]>;
  designMode: Ref<boolean>;
  eventLog: Ref<LogEntry[]>;
  maxConcurrency: Ref<number>;
  primaryAgent: { value: string };
  setView: (view: PipelineView | null, metadata?: { templateId?: string }) => void;
  stopCurrentPolling: () => void;
  refreshList: () => Promise<void>;
  setError: (tool: string, err: unknown) => void;
  withError: <T>(tool: string, fn: () => Promise<T>) => Promise<T | undefined>;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T>;
}

export function createPipelineDesignActions(deps: DesignActionsDeps) {
  const {
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
  } = deps;

  // unused `currentView` and `pipelines` would be needed later; keep optional silent
  void currentView;
  void pipelines;

  function appendNode(node: PipelineNode): PipelineNode {
    localDefinition.value = {
      ...localDefinition.value,
      nodes: [...localDefinition.value.nodes, node],
    };
    return node;
  }

  function addNode(node: PipelineNode): PipelineNode;
  function addNode(type: 'workflow', fields: WorkflowNodeInput): PipelineNode;
  function addNode(type: 'prompt', fields: PromptNodeInput): PipelineNode;
  function addNode(type: 'sequentialThinking', fields: SequentialThinkingNodeInput): PipelineNode;
  function addNode(type: 'tool', fields: ToolNodeInput): PipelineNode;
  function addNode(
    nodeOrType: PipelineNode | PipelineNodeType,
    fields?: PipelineNodeInput,
  ): PipelineNode {
    if (typeof nodeOrType !== 'string') {
      return appendNode(nodeOrType);
    }
    if (!fields) {
      throw new Error(`Missing fields for ${nodeOrType} node`);
    }
    return appendNode(createNode(nodeOrType, fields));
  }

  async function loadPipeline(pipelineId: string): Promise<boolean> {
    const view = await withError('view_pipeline', async () =>
      normalizeView(await mcp.viewPipeline(pipelineId)),
    );
    if (!view) return false;

    stopCurrentPolling();
    currentPipelineId.value = pipelineId;
    setView(view);
    designMode.value = false;
    eventLog.value = hydrateEventLogFromView(view);
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
      if (
        isCreateResult(result)
        && typeof result.pipelineId === 'string'
        && result.pipelineId.length > 0
      ) {
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

  async function applyTemplate(template: PipelineTemplate): Promise<boolean> {
    const agents = resolveAgents(
      configStore.agentNames,
      resolvePreferredAgent(configStore.agentNames),
    );
    const agent = agents.primaryAgent.trim();
    if (agent.length === 0) {
      setError('apply_template', new Error('No agent is available for template-based creation. Configure an agent before creating a pipeline.'));
      return false;
    }

    const result = instantiateTemplate(template, agents);
    localDefinition.value = withLocalMetadata(result.definition, { templateId: template.id });
    return true;
  }

  async function createIntentPipeline(intent: string): Promise<boolean> {
    const prompt = intent.trim();
    if (prompt.length === 0) return false;

    const agent = primaryAgent.value;
    if (agent.length === 0) {
      setError('create_intent_pipeline', new Error('No agent is available for intent-based creation. Configure an agent before creating a pipeline.'));
      return false;
    }

    localDefinition.value = buildIntentDefinition(prompt, agent);
    return true;
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

  return {
    addNode,
    loadPipeline,
    newPipeline,
    createPipeline,
    applyTemplate,
    createIntentPipeline,
    connect,
    disconnect,
  };
}
