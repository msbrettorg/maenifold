import type { Node } from '@vue-flow/core';

import type { PipelineNode } from './pipeline-types-node';
import type { ViewNode } from './pipeline-types-view';
import { getNodeShellClassName, getNodeShellStyle } from './pipeline-node-status';
import {
  loadPositions as loadPositionsImpl,
  savePositions as savePositionsImpl,
} from './pipeline-composer-flow-nodes-persistence';
import {
  toFlowNodeData as toFlowNodeDataImpl,
  toPipelineNode as toPipelineNodeImpl,
  toPipelineNodeFromViewNode as toPipelineNodeFromViewNodeImpl,
} from './pipeline-composer-flow-nodes-conversion';

export type FlowNodeData = import('./pipeline-composer-flow-nodes-conversion').FlowNodeData;

export function loadPositions(
  pipelineId?: string,
): Record<string, { x: number; y: number }> {
  return loadPositionsImpl(pipelineId);
}

export function savePositions(
  flowNodes: Node[],
  pipelineId?: string,
): void {
  savePositionsImpl(flowNodes, pipelineId);
}

export function toFlowNodeData(
  node: PipelineNode,
  viewNode?: ViewNode,
): FlowNodeData {
  return toFlowNodeDataImpl(node, viewNode);
}

export function toPipelineNode(data: FlowNodeData): PipelineNode {
  return toPipelineNodeImpl(data);
}

export function toPipelineNodeFromViewNode(node: ViewNode): PipelineNode {
  return toPipelineNodeFromViewNodeImpl(node);
}

export function pipelineNodesToFlow(
  pipelineNodes: PipelineNode[],
  pipelineId?: string,
  viewNodes?: ViewNode[],
): Node[] {
  const cached = loadPositions(pipelineId);
  const viewMap = new Map(viewNodes?.map((node) => [node.nodeId, node]) ?? []);
  return pipelineNodes.map((node, index) => ({
    id: node.id,
    type: 'pipelineNode',
    class: getNodeShellClassName(viewMap.get(node.id)?.status),
    position: cached[node.id] ?? {
      x: (index % 4) * 340 + 40,
      y: Math.floor(index / 4) * 240 + 40,
    },
    style: getNodeShellStyle(viewMap.get(node.id)?.status),
    data: toFlowNodeData(node, viewMap.get(node.id)),
  }));
}
