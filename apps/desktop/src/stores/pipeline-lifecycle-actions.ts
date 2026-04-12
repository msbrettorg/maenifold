import type { Ref } from 'vue';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import type { PipelineEdge, PipelineInputs, PipelineNode } from '../lib/pipeline-types';

type PipelineMcp = ReturnType<typeof usePipelineMcp>;
type WithError = <T>(tool: string, fn: () => Promise<T>) => Promise<T | undefined>;
type RefreshView = (options?: { resetEventLog?: boolean }) => Promise<void>;

interface PipelineLifecycleActionDependencies {
  mcp: PipelineMcp;
  currentPipelineId: Ref<string | null>;
  refreshView: RefreshView;
  refreshList: () => Promise<void>;
  startViewPolling: () => void;
  stopCurrentPolling: () => void;
  newPipeline: () => void;
  withError: WithError;
  serializeEdge: (edge: PipelineEdge) => PipelineEdge;
}

export function createPipelineLifecycleActions({
  mcp,
  currentPipelineId,
  refreshView,
  refreshList,
  startViewPolling,
  stopCurrentPolling,
  newPipeline,
  withError,
  serializeEdge,
}: PipelineLifecycleActionDependencies) {
  async function withCurrentPipeline(
    tool: string,
    action: (sessionId: string) => Promise<void>,
  ): Promise<void | undefined> {
    const sessionId = currentPipelineId.value;
    if (!sessionId) return;

    return withError(tool, async () => {
      await action(sessionId);
    });
  }

  async function runPipeline(prompt?: string, inputs?: PipelineInputs) {
    await withCurrentPipeline('run_pipeline', async (sessionId) => {
      await mcp.runPipeline(sessionId, prompt, inputs);
      startViewPolling();
      await refreshView();
    });
  }

  async function pausePipeline() {
    await withCurrentPipeline('pause_pipeline', async (sessionId) => {
      await mcp.pausePipeline(sessionId);
      stopCurrentPolling();
      await refreshView();
    });
  }

  async function resumePipeline() {
    await withCurrentPipeline('resume_pipeline', async (sessionId) => {
      await mcp.resumePipeline(sessionId);
      startViewPolling();
      await refreshView();
    });
  }

  async function cancelPipeline() {
    await withCurrentPipeline('cancel_pipeline', async (sessionId) => {
      await mcp.cancelPipeline(sessionId);
      stopCurrentPolling();
      await refreshView();
    });
  }

  async function deletePipeline() {
    const sessionId = currentPipelineId.value;
    if (!sessionId) return;
    await deletePipelineById(sessionId);
  }

  async function deletePipelineById(sessionId: string) {
    await withError('delete_pipeline', async () => {
      await mcp.deletePipeline(sessionId);
      if (currentPipelineId.value === sessionId) {
        newPipeline();
      }
      await refreshList();
    });
  }

  async function rewindNodes(nodeIds: string[]) {
    await withCurrentPipeline('rewind_nodes', async (sessionId) => {
      await mcp.rewindPipeline(sessionId, nodeIds);
      startViewPolling();
      await refreshView({ resetEventLog: true });
    });
  }

  async function restartNode(nodeId: string) {
    await withCurrentPipeline('restart_node', async (sessionId) => {
      await mcp.restartNode(sessionId, nodeId);
      startViewPolling();
      await refreshView({ resetEventLog: true });
    });
  }

  async function updateNodeStatus(
    nodeId: string,
    status: 'completed' | 'abandoned',
    conclusion?: string,
  ) {
    await withCurrentPipeline('update_node_status', async (sessionId) => {
      await mcp.updateNodeStatus(sessionId, nodeId, status, conclusion);
      await refreshView();
    });
  }

  async function graftNodes(nodes: PipelineNode[], edges: PipelineEdge[]) {
    await withCurrentPipeline('graft_pipeline', async (sessionId) => {
      await mcp.graftPipeline(
        sessionId,
        nodes.map((node) => ({ ...node })),
        edges.map(serializeEdge),
      );
      await refreshView();
    });
  }

  return {
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
  };
}
