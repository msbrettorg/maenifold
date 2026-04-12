import type { Ref } from 'vue';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import type { PipelineEdge } from '../lib/pipeline-types';

type PipelineMcp = ReturnType<typeof usePipelineMcp>;
type WithError = <T>(tool: string, fn: () => Promise<T>) => Promise<T | undefined>;

interface PipelineMutationActionDependencies {
  mcp: PipelineMcp;
  currentPipelineId: Ref<string | null>;
  refreshView: () => Promise<void>;
  refreshList: () => Promise<void>;
  startViewPolling: () => void;
  loadPipeline: (pipelineId: string) => Promise<boolean>;
  withError: WithError;
  setSuccessMessage: (message: string | null) => void;
}

function extractPipelineId(result: unknown): string | null {
  if (typeof result === 'string') {
    return result.trim() || null;
  }

  if (!result || typeof result !== 'object') {
    return null;
  }

  const record = result as Record<string, unknown>;
  for (const key of ['pipelineId', 'sessionId', 'newSessionId', 'forkedSessionId']) {
    const value = record[key];
    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

export function createPipelineMutationActions({
  mcp,
  currentPipelineId,
  refreshView,
  refreshList,
  startViewPolling,
  loadPipeline,
  withError,
  setSuccessMessage,
}: PipelineMutationActionDependencies) {
  async function refreshCurrentSession(sessionId: string, shouldStartPolling = false) {
    if (currentPipelineId.value !== sessionId) return;
    if (shouldStartPolling) startViewPolling();
    await refreshView();
  }

  async function forkPipeline(sessionId: string, fromNodeId: string, newSessionId?: string) {
    return withError('fork_pipeline', async () => {
      const result = await mcp.forkPipeline(sessionId, fromNodeId, newSessionId);
      await refreshList();
      const forkedPipelineId = extractPipelineId(result);
      if (forkedPipelineId) {
        const loaded = await loadPipeline(forkedPipelineId);
        if (!loaded) {
          await refreshCurrentSession(sessionId);
        }
        setSuccessMessage(`Forked pipeline ${forkedPipelineId}.`);
      } else {
        await refreshCurrentSession(sessionId);
        setSuccessMessage('Forked pipeline.');
      }
      return result;
    });
  }

  async function joinPipeline(sessionId: string, sourceSessionId: string, edges: PipelineEdge[]) {
    return withError('join_pipeline', async () => {
      const result = await mcp.joinPipeline(sessionId, sourceSessionId, edges);
      await refreshCurrentSession(sessionId);
      await refreshList();
      return result;
    });
  }

  async function dispatchPipeline(sessionId: string) {
    return withError('dispatch_pipeline', async () => {
      const result = await mcp.dispatchPipeline(sessionId);
      await refreshCurrentSession(sessionId, true);
      await refreshList();
      return result;
    });
  }

  return { forkPipeline, joinPipeline, dispatchPipeline };
}
