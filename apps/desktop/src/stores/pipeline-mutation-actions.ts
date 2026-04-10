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
  withError: WithError;
}

export function createPipelineMutationActions({
  mcp,
  currentPipelineId,
  refreshView,
  refreshList,
  startViewPolling,
  withError,
}: PipelineMutationActionDependencies) {
  async function refreshCurrentSession(sessionId: string, shouldStartPolling = false) {
    if (currentPipelineId.value !== sessionId) return;
    if (shouldStartPolling) startViewPolling();
    await refreshView();
  }

  async function forkPipeline(sessionId: string, fromNodeId: string, newSessionId?: string) {
    return withError('fork_pipeline', async () => {
      const result = await mcp.forkPipeline(sessionId, fromNodeId, newSessionId);
      await refreshCurrentSession(sessionId);
      await refreshList();
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
