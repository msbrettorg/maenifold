// Re-export shim for backward compatibility.
// Types are split across pipeline-types-node.ts (PipelineNode + guards),
// pipeline-types-edge.ts (PipelineEdge + PipelineDefinition + status enums),
// and pipeline-types-view.ts (server-side ViewNode + PipelineView).

export type {
  PipelineNodeType,
  PipelineNodeAgentConfig,
  PipelineNodeAdopt,
  WorkflowNode,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  PipelineNode,
} from "./pipeline-types-node";

export {
  getNodeType,
  isWorkflowNode,
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  getPipelineNodeDisplayLabel,
  nodeLabel,
  cwdHasReasonablePathPrefix,
  normalizeNodeCwd,
} from "./pipeline-types-node";

export type {
  EdgeType,
  EdgeConditionEquals,
  EdgeCondition,
  PipelineEdge,
  PipelineInputs,
  PipelineDefinition,
  NodeStatus,
  PipelineStatus,
} from "./pipeline-types-edge";

export type {
  NodeProvenanceDetail,
  NodeProvenance,
  ToolProvenance,
  WorkflowStepEntry,
  WorkflowViewNode,
  PromptViewNode,
  SequentialThinkingViewNode,
  ToolViewNode,
  ViewNode,
  PipelineView,
  PipelineListItem,
  LogEntry,
} from "./pipeline-types-view";

export { getViewNodeDisplayLabel } from "./pipeline-types-view";

export type {
  PipelineNodeByType,
  PipelineNodeDraftByType,
  PipelineNodeInput,
  PipelineNodeInputByType,
  PromptNodeInput,
  SequentialThinkingNodeInput,
  ToolNodeInput,
  WorkflowNodeInput,
} from "./pipeline-node-factory";
export { createNode, createNodeId } from "./pipeline-node-factory";
