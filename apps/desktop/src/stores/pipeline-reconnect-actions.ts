import type { Ref } from 'vue';
import type { PipelineView, ViewNode } from '../lib/pipeline-types';

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

  function markNodeReconnected(nodeId: string): void {
    const node = currentView.value ? findNode(currentView.value, nodeId) : undefined;
    if (!node || node.type === 'tool') return;
    node.reconnected = true;
  }

  function clearNodeReconnected(nodeId: string): void {
    const node = currentView.value ? findNode(currentView.value, nodeId) : undefined;
    if (node) node.reconnected = false;
  }

  function replaceNodeOutput(
    nodeId: string,
    output: string,
    metadata: ReplaceNodeOutputMetadata,
  ): void {
    const view = currentView.value;
    const node = view ? findNode(view, nodeId) : undefined;
    if (!view || !node) return;
    node.output = output;
    node.reconnected = false;
    node.refinement = metadata;
    const staleNodeIds = downstreamNodeIds(view, nodeId);
    view.nodes.forEach((candidate) => {
      if (staleNodeIds.has(candidate.nodeId)) candidate.stale = true;
    });
  }

  return {
    markNodeReconnected,
    clearNodeReconnected,
    replaceNodeOutput,
  };
}
