import type { Ref } from 'vue';
import type { PipelineView, ViewNode } from '../lib/pipeline-types-view';

interface ReplaceNodeOutputMetadata {
  sessionId: string;
  takenAt: number;
}

interface ReconnectActionsDeps {
  currentView: Ref<PipelineView | null>;
}

function findNode(view: PipelineView, nodeId: string): ViewNode | undefined {
  return view.nodes.find((node) => node.nodeId === nodeId);
}

function downstreamNodeIds(view: PipelineView, nodeId: string): Set<string> {
  const downstream = new Set<string>();
  const pending = [nodeId];
  while (pending.length > 0) {
    const source = pending.shift()!;
    for (const edge of view.edges) {
      if (edge.source !== source || downstream.has(edge.target)) continue;
      downstream.add(edge.target);
      pending.push(edge.target);
    }
  }
  downstream.delete(nodeId);
  return downstream;
}

export function createPipelineReconnectActions(deps: ReconnectActionsDeps) {
  const { currentView } = deps;

  function updateNode(nodeId: string, update: (node: ViewNode) => ViewNode): void {
    const view = currentView.value;
    const node = view ? findNode(view, nodeId) : undefined;
    if (!view || !node) return;
    currentView.value = {
      ...view,
      nodes: view.nodes.map((candidate) =>
        candidate.nodeId === nodeId ? update(candidate) : candidate,
      ),
    };
  }

  function markNodeReconnected(nodeId: string): void {
    updateNode(nodeId, (node) => (
      node.type === 'tool' ? node : { ...node, reconnected: true }
    ));
  }

  function clearNodeReconnected(nodeId: string): void {
    updateNode(nodeId, (node) => ({ ...node, reconnected: false }));
  }

  function replaceNodeOutput(
    nodeId: string,
    output: string,
    metadata: ReplaceNodeOutputMetadata,
  ): void {
    const view = currentView.value;
    const node = view ? findNode(view, nodeId) : undefined;
    if (!view || !node) return;
    const staleNodeIds = downstreamNodeIds(view, nodeId);
    currentView.value = {
      ...view,
      nodes: view.nodes.map((candidate) => {
        if (candidate.nodeId === nodeId) {
          return {
            ...candidate,
            output,
            reconnected: false,
            refinement: metadata,
          };
        }
        return staleNodeIds.has(candidate.nodeId)
          ? { ...candidate, stale: true }
          : candidate;
      }),
    };
  }

  return {
    markNodeReconnected,
    clearNodeReconnected,
    replaceNodeOutput,
  };
}
