import { computed, ref } from 'vue';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineInputs,
} from '../lib/pipeline-types-edge';
import type { PipelineNode } from '../lib/pipeline-types-node';
import type {
  PipelineListItem,
  PipelineView,
} from '../lib/pipeline-types-view';
import type { WorkflowAssetDefinition, WorkflowAssetSummary } from '../lib/workflow-asset-types';
import { createPipelinePolling } from './pipeline-mcp-polling';
import { normalizeWorkflowIds, unwrapEnvelope, unwrapToolData } from './pipeline-mcp-transport';
import { diffPipelineViews } from './pipeline-view-diff';
import { getSharedMcpClient } from './useSharedMcpClient';

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
    return mcp.gatewayExecute('maenifold.workflow', 'Pipeline', payload).then((result) => {
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

  function updatePipeline(
    sessionId: string,
    definition: PipelineDefinition,
    options: PipelineCreateOptions = {},
  ) {
    const name = options.name?.trim();
    const description = options.description?.trim();
    return callPipeline({
      sessionId,
      definition,
      ...(name ? { name } : {}),
      ...(description ? { description } : {}),
      update: true,
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
    return callPipeline({ sessionId, resume: true });
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
    return mcp.gatewayExecute('maenifold.system', 'ListAssets', { type: 'workflow' }).then((result) => normalizeWorkflowIds(result));
  }

  function listWorkflowSummaries(): Promise<WorkflowAssetSummary[]> {
    return mcp.gatewayExecute('maenifold.system', 'ListAssets', { type: 'workflow' }).then((result) => {
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
    return mcp.gatewayExecute('maenifold.system', 'ListAssets', { type }).then((result) => {
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
    gatewayExecute: mcp.gatewayExecute,
    callPipeline,
    listPipelines,
    createPipeline,
    updatePipeline,
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
