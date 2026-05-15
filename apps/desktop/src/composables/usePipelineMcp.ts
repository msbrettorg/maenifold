import { computed, ref } from 'vue';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineInputs,
  PipelineListItem,
  PipelineNode,
  PipelineView,
} from '../lib/pipeline-types';
import type { WorkflowAssetDefinition, WorkflowAssetSummary } from '../lib/workflow-asset-types';
import { createPipelinePolling } from './pipeline-mcp-polling';
import { normalizeWorkflowIds, unwrapEnvelope, unwrapToolData } from './pipeline-mcp-transport';
import { diffPipelineViews } from './pipeline-view-diff';
import { useMcpClient } from './useMcpClient';

interface PipelineListResult {
  pipelines?: PipelineListItem[];
}

export type PipelineAssetType = 'workflow' | 'role' | 'color' | 'perspective';

export interface PipelineAssetSummary {
  id: string;
  name?: string;
  emoji?: string;
  description?: string;
  multiAgent?: boolean;
}

export interface PipelineCreateOptions {
  name?: string;
  description?: string;
}

const sharedMcpClients = new Map<string, ReturnType<typeof useMcpClient>>();

export function getSharedMcpClient(baseUrl?: string) {
  const key = baseUrl ?? '__default__';
  const existing = sharedMcpClients.get(key);
  if (existing) {
    return existing;
  }

  const client = useMcpClient(baseUrl);
  sharedMcpClients.set(key, client);
  return client;
}

export function usePipelineMcp(baseUrl?: string) {
  const mcp = getSharedMcpClient(baseUrl);
  const pollingError = ref<string | null>(null);
  const error = computed(() => pollingError.value ?? mcp.error.value);
  const startPolling = createPipelinePolling({
    pollingError,
    reconnecting: mcp.reconnecting,
    viewPipeline,
  });
  const diffViews = diffPipelineViews;

  function callPipeline<T = unknown>(payload: Record<string, unknown>): Promise<T> {
    return mcp.callTool('Pipeline', payload).then((result) => {
      return unwrapEnvelope<T>(result);
    });
  }

  function listPipelines(): Promise<PipelineListResult> {
    return callPipeline<PipelineListResult>({ list: true });
  }

  function createPipeline(
    definition: PipelineDefinition,
    maxConcurrency = 8,
    options: PipelineCreateOptions = {},
  ) {
    const name = options.name?.trim();
    const description = options.description?.trim();
    return callPipeline({
      definition,
      maxConcurrency,
      ...(name ? { name } : {}),
      ...(description ? { description } : {}),
    });
  }

  function viewPipeline(sessionId: string): Promise<PipelineView> {
    return callPipeline<PipelineView>({ sessionId, view: true });
  }

  function runPipeline(sessionId: string, prompt?: string, inputs?: PipelineInputs) {
    return callPipeline({
      sessionId,
      run: true,
      ...(prompt ? { prompt } : {}),
      ...(inputs && Object.keys(inputs).length > 0 ? { inputs } : {}),
    });
  }

  function pausePipeline(sessionId: string) {
    return callPipeline({ sessionId, pause: true });
  }

  function resumePipeline(sessionId: string) {
    return callPipeline({ sessionId, run: true });
  }

  function cancelPipeline(sessionId: string) {
    return callPipeline({ sessionId, cancel: true });
  }

  function deletePipeline(sessionId: string) {
    return callPipeline({ sessionId, delete: true });
  }

  function graftPipeline(sessionId: string, nodes: PipelineNode[], edges: PipelineEdge[]) {
    return callPipeline({ sessionId, graft: { nodes, edges } });
  }

  function rewindPipeline(sessionId: string, nodeIds: string[]) {
    return callPipeline({ sessionId, rewind: { nodeIds } });
  }

  function forkPipeline(sessionId: string, fromNodeId: string, newSessionId?: string) {
    return callPipeline({
      sessionId,
      fork: newSessionId ? { fromNodeId, newSessionId } : fromNodeId,
    });
  }

  function joinPipeline(sessionId: string, sourceSessionId: string, edges: PipelineEdge[]) {
    return callPipeline({ sessionId, join: { sourceSessionId, edges } });
  }

  function restartNode(sessionId: string, nodeId: string) {
    return callPipeline({ sessionId, nodeId, restart: true });
  }

  function updateNodeStatus(
    sessionId: string,
    nodeId: string,
    status: 'completed' | 'abandoned',
    conclusion?: string,
  ) {
    return callPipeline({
      sessionId,
      nodeId,
      status,
      ...(conclusion ? { conclusion } : {}),
    });
  }

  function dispatchPipeline(sessionId: string) {
    return callPipeline({ sessionId, dispatch: true });
  }

  function listWorkflows(): Promise<string[]> {
    return mcp.callTool('ListAssets', { type: 'workflow' }).then((result) => normalizeWorkflowIds(result));
  }

  function listWorkflowSummaries(): Promise<WorkflowAssetSummary[]> {
    return mcp.callTool('ListAssets', { type: 'workflow' }).then((result) => {
      return unwrapToolData<WorkflowAssetSummary[]>(result);
    });
  }

  function readWorkflowDefinition(id: string): Promise<WorkflowAssetDefinition> {
    return mcp.callTool('ReadMcpResource', { uri: `asset://workflows/${id}` }).then((result) => {
      return unwrapToolData<WorkflowAssetDefinition>(result);
    });
  }

  function listTools() {
    return mcp.listTools().then((result) => {
      return result;
    });
  }

  function listAssets(type: PipelineAssetType): Promise<PipelineAssetSummary[]> {
    return mcp.callTool('ListAssets', { type }).then((result) => {
      return unwrapToolData<PipelineAssetSummary[]>(result);
    });
  }

  return {
    connect: mcp.connect,
    disconnect: mcp.disconnect,
    connected: mcp.connected,
    reconnecting: mcp.reconnecting,
    error,
    callTool: mcp.callTool,
    callPipeline,
    listPipelines,
    createPipeline,
    viewPipeline,
    runPipeline,
    pausePipeline,
    resumePipeline,
    cancelPipeline,
    deletePipeline,
    graftPipeline,
    rewindPipeline,
    forkPipeline,
    joinPipeline,
    restartNode,
    updateNodeStatus,
    dispatchPipeline,
    listWorkflows,
    listWorkflowSummaries,
    readWorkflowDefinition,
    listTools,
    listAssets,
    startPolling,
    diffViews,
  };
}
