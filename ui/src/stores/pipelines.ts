import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { executeGateway, readGatewayResource } from '../lib/mcp/gateway';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineInputs,
  PipelineListItem,
  PipelineListResponse,
  PipelineNode,
  PipelineSessionsResource,
  PipelineStatus,
  PipelineView,
  ViewNode,
} from '../lib/pipeline-types';

// The Pipeline tool lives in the `maenifold.workflow` MCP domain and is a
// single multiplexed command: the operation is selected by which flag
// (`list`/`view`/`run`/...) is present in the parameters. See the backend
// param schema in crates/maenifold-core/src/mcp/schema/pipeline_params.rs.
const WORKFLOW_DOMAIN = 'maenifold.workflow';
const PIPELINE_COMMAND = 'Pipeline';
const SESSIONS_URI = 'pipeline://sessions';
const DEFAULT_MAX_CONCURRENCY = 8;

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

// --- snake_case → camelCase normalization (done ONCE at the boundary) -------
// The backend emits camelCase for almost every field, but the cognitive
// surfaces (`assembled_prompt`, `workflow_steps`, `thinking_trace`, and their
// members) are snake_case. We renormalize them here so the rest of the app
// reads one convention. No dual-read shims live downstream of this file.

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function renameKeys(
  source: Record<string, unknown>,
  renames: Record<string, string>
): Record<string, unknown> {
  const next: Record<string, unknown> = { ...source };
  for (const [from, to] of Object.entries(renames)) {
    if (from in next) {
      if (next[from] !== undefined && !(to in next)) {
        next[to] = next[from];
      }
      delete next[from];
    }
  }
  return next;
}

function normalizeWorkflowSteps(value: unknown): unknown {
  if (!Array.isArray(value)) return value;
  return value.map((entry) =>
    isRecord(entry) ? renameKeys(entry, { step_name: 'stepName' }) : entry
  );
}

function normalizeThinkingTrace(value: unknown): unknown {
  if (!Array.isArray(value)) return value;
  return value.map((entry) =>
    isRecord(entry)
      ? renameKeys(entry, {
          thought_number: 'thoughtNumber',
          total_thoughts: 'totalThoughts',
          is_revision: 'isRevision',
          revises_thought: 'revisesThought',
          branch_id: 'branchId',
        })
      : entry
  );
}

function normalizeViewNode(raw: unknown): ViewNode {
  if (!isRecord(raw)) return raw as ViewNode;
  const node = renameKeys(raw, {
    assembled_prompt: 'assembledPrompt',
    workflow_steps: 'workflowSteps',
    thinking_trace: 'thinkingTrace',
  });
  if ('workflowSteps' in node) {
    node.workflowSteps = normalizeWorkflowSteps(node.workflowSteps);
  }
  if ('thinkingTrace' in node) {
    node.thinkingTrace = normalizeThinkingTrace(node.thinkingTrace);
  }
  // One assertion here: after the snake_case→camelCase key renames above the
  // record matches a `ViewNode` member by construction. We do NOT runtime-
  // validate the discriminated union or coerce every field — a full validator
  // is out of this foundation task's scope — so this is an assertion, not a
  // guarantee, and it is the single cast in the node path.
  return node as unknown as ViewNode;
}

function asStringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : [];
}

function normalizePipelineView(raw: unknown): PipelineView {
  if (!isRecord(raw)) {
    throw new Error('Pipeline view returned a non-object payload');
  }
  // Build the typed view field-by-field so the only casts are narrow,
  // field-level assertions (status/inputs/schema) rather than one blanket
  // `as unknown as PipelineView` over the whole payload.
  return {
    pipelineId: typeof raw.pipelineId === 'string' ? raw.pipelineId : '',
    name: typeof raw.name === 'string' ? raw.name : null,
    description: typeof raw.description === 'string' ? raw.description : null,
    status: raw.status as PipelineStatus,
    final: typeof raw.final === 'boolean' ? raw.final : undefined,
    templateId: typeof raw.templateId === 'string' ? raw.templateId : undefined,
    inputs: isRecord(raw.inputs) ? (raw.inputs as PipelineInputs) : undefined,
    inputSchema: isRecord(raw.inputSchema) ? (raw.inputSchema as Record<string, unknown>) : undefined,
    nodes: Array.isArray(raw.nodes) ? raw.nodes.map(normalizeViewNode) : [],
    edges: Array.isArray(raw.edges) ? (raw.edges as PipelineEdge[]) : [],
    ready: asStringArray(raw.ready),
    active: asStringArray(raw.active),
    completed: asStringArray(raw.completed),
    mermaid: typeof raw.mermaid === 'string' ? raw.mermaid : '',
    running: typeof raw.running === 'boolean' ? raw.running : false,
  };
}

function normalizeListItems(raw: unknown): PipelineListItem[] {
  if (!Array.isArray(raw)) return [];
  return raw.filter(
    (entry): entry is PipelineListItem =>
      isRecord(entry) && typeof entry.pipelineId === 'string'
  );
}

export const usePipelinesStore = defineStore('pipelines', () => {
  const pipelines = ref<PipelineListItem[]>([]);
  const selectedView = ref<PipelineView | null>(null);
  const selectedPipelineId = ref<string | null>(null);
  const bootstrapped = ref(false);
  const loading = ref(false);
  const viewLoading = ref(false);
  const error = ref<string | null>(null);

  const pipelineCount = computed(() => pipelines.value.length);
  const selectedNodes = computed<ViewNode[]>(() => selectedView.value?.nodes ?? []);
  const selectedEdges = computed<PipelineEdge[]>(() => selectedView.value?.edges ?? []);

  // --- reads ----------------------------------------------------------------

  // Preferred read path: the `pipeline://sessions` MCP resource (bypasses
  // gateway schema validation and matches the readGatewayResource house style).
  async function refreshList(): Promise<PipelineListItem[]> {
    loading.value = true;
    try {
      const resource = await readGatewayResource<PipelineSessionsResource>(SESSIONS_URI);
      const items = normalizeListItems(resource?.sessions);
      pipelines.value = items;
      error.value = null;
      return items;
    } catch (nextError) {
      error.value = toErrorMessage(nextError);
      throw nextError;
    } finally {
      loading.value = false;
    }
  }

  // The `{ list: true }` tool operation. Same data as the resource, reached
  // through the gateway command surface.
  async function listPipelines(): Promise<PipelineListItem[]> {
    const response = await executeGateway<PipelineListResponse>(
      WORKFLOW_DOMAIN,
      PIPELINE_COMMAND,
      { list: true }
    );
    return normalizeListItems(response?.pipelines);
  }

  async function viewPipeline(sessionId: string): Promise<PipelineView> {
    viewLoading.value = true;
    try {
      const raw = await executeGateway<Record<string, unknown>>(
        WORKFLOW_DOMAIN,
        PIPELINE_COMMAND,
        { sessionId, view: true }
      );
      const view = normalizePipelineView(raw);
      selectedView.value = view;
      selectedPipelineId.value = view.pipelineId ?? sessionId;
      error.value = null;
      return view;
    } catch (nextError) {
      error.value = toErrorMessage(nextError);
      throw nextError;
    } finally {
      viewLoading.value = false;
    }
  }

  // --- lifecycle mutations --------------------------------------------------

  async function createPipeline(
    definition: PipelineDefinition,
    options: {
      maxConcurrency?: number;
      name?: string;
      description?: string;
    } = {}
  ): Promise<unknown> {
    const name = options.name?.trim();
    const description = options.description?.trim();
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      definition,
      maxConcurrency: options.maxConcurrency ?? DEFAULT_MAX_CONCURRENCY,
      ...(name ? { name } : {}),
      ...(description ? { description } : {}),
    });
  }

  async function updatePipeline(
    sessionId: string,
    definition: PipelineDefinition,
    options: { name?: string; description?: string } = {}
  ): Promise<unknown> {
    const name = options.name?.trim();
    const description = options.description?.trim();
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      definition,
      ...(name ? { name } : {}),
      ...(description ? { description } : {}),
      update: true,
    });
  }

  async function runPipeline(
    sessionId: string,
    options: { prompt?: string; inputs?: PipelineInputs } = {}
  ): Promise<unknown> {
    const prompt = options.prompt?.trim();
    const inputs = options.inputs;
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      run: true,
      ...(prompt ? { prompt } : {}),
      ...(inputs && Object.keys(inputs).length > 0 ? { inputs } : {}),
    });
  }

  async function pausePipeline(sessionId: string): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      pause: true,
    });
  }

  async function resumePipeline(sessionId: string): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      resume: true,
    });
  }

  async function cancelPipeline(sessionId: string): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      cancel: true,
    });
  }

  async function deletePipeline(sessionId: string, force = false): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      delete: true,
      ...(force ? { force: true } : {}),
    });
  }

  async function dispatchPipeline(sessionId: string): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      dispatch: true,
    });
  }

  // --- graph surgery --------------------------------------------------------

  async function rewindNodes(sessionId: string, nodeIds: string[]): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      rewind: { nodeIds },
    });
  }

  async function graftNodes(
    sessionId: string,
    nodes: PipelineNode[],
    edges: PipelineEdge[]
  ): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      graft: { nodes, edges },
    });
  }

  async function joinPipeline(
    sessionId: string,
    sourceSessionId: string,
    edges: PipelineEdge[]
  ): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      join: { sourceSessionId, edges },
    });
  }

  async function forkPipeline(
    sessionId: string,
    fromNodeId: string,
    newSessionId?: string
  ): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      fork: newSessionId ? { fromNodeId, newSessionId } : fromNodeId,
    });
  }

  // --- node-level operations ------------------------------------------------

  async function restartNode(sessionId: string, nodeId: string): Promise<unknown> {
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      nodeId,
      restart: true,
    });
  }

  async function setNodeStatus(
    sessionId: string,
    nodeId: string,
    status: 'completed' | 'abandoned',
    conclusion?: string
  ): Promise<unknown> {
    const trimmed = conclusion?.trim();
    return executeGateway<unknown>(WORKFLOW_DOMAIN, PIPELINE_COMMAND, {
      sessionId,
      nodeId,
      status,
      ...(trimmed ? { conclusion: trimmed } : {}),
    });
  }

  // --- bootstrap ------------------------------------------------------------

  async function refreshAll(): Promise<void> {
    await refreshList();
    bootstrapped.value = true;
  }

  async function bootstrap(): Promise<void> {
    if (bootstrapped.value) return;
    await refreshAll();
  }

  return {
    pipelines,
    selectedView,
    selectedPipelineId,
    bootstrapped,
    loading,
    viewLoading,
    error,
    pipelineCount,
    selectedNodes,
    selectedEdges,
    refreshList,
    listPipelines,
    viewPipeline,
    createPipeline,
    updatePipeline,
    runPipeline,
    pausePipeline,
    resumePipeline,
    cancelPipeline,
    deletePipeline,
    dispatchPipeline,
    rewindNodes,
    graftNodes,
    joinPipeline,
    forkPipeline,
    restartNode,
    setNodeStatus,
    refreshAll,
    bootstrap,
  };
});
