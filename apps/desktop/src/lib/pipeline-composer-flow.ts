// Re-export shim. Implementation split across:
//   pipeline-composer-flow-nodes.ts (FlowNodeData + node converters + position cache)
//   pipeline-composer-flow-edges.ts (edge converters + definitionFromFlow)
export type { FlowNodeData } from './pipeline-composer-flow-nodes';
export {
  loadPositions,
  savePositions,
  toFlowNodeData,
  toPipelineNode,
  toPipelineNodeFromViewNode,
  pipelineNodesToFlow,
} from './pipeline-composer-flow-nodes';
export {
  pipelineEdgesToFlow,
  definitionFromFlow,
} from './pipeline-composer-flow-edges';
