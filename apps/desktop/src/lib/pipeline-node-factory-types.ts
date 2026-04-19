// Input/draft type definitions for pipeline node factory.
// Extracted from pipeline-node-factory.ts to keep modules under 300 LOC.
import type {
  PipelineNodeAdopt,
  PipelineNodeAgentConfig,
  PipelineNode,
  PipelineNodeType,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';

export interface PipelineNodeExecutionFields {
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

export interface WorkflowNodeExecutionSelection {
  model?: string;
  thoughtLevel?: string;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

export interface WorkflowNodeInput extends WorkflowNodeExecutionSelection {
  id: string;
  displayLabel?: string;
  workflowId: string;
  acpClient?: string;
  agent?: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  prompt?: string;
}

export interface PromptNodeInput extends PipelineNodeExecutionFields {
  id: string;
  displayLabel?: string;
  prompt: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  toolMode?: string;
}

export interface SequentialThinkingNodeInput extends PipelineNodeExecutionFields {
  id: string;
  displayLabel?: string;
  topic: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  totalThoughts?: number;
  analysisType?: string;
}

export interface ToolNodeInput extends PipelineNodeExecutionFields {
  id: string;
  displayLabel?: string;
  toolName: string;
  agent?: string;
  toolArgs?: Record<string, unknown>;
}

export interface PipelineNodeInputByType {
  workflow: WorkflowNodeInput;
  prompt: PromptNodeInput;
  sequentialThinking: SequentialThinkingNodeInput;
  tool: ToolNodeInput;
}

export interface PipelineNodeByType {
  workflow: WorkflowNode;
  prompt: PromptNode;
  sequentialThinking: SequentialThinkingNode;
  tool: ToolNode;
}

export type PipelineNodeInput = PipelineNodeInputByType[PipelineNodeType];
export type PipelineNodeDraftByType = {
  [Type in PipelineNodeType]: Omit<PipelineNodeInputByType[Type], 'id'>;
};

export type AnyDisplayLabelPick =
  | Pick<WorkflowNode, 'displayLabel'>
  | Pick<PromptNode, 'displayLabel'>
  | Pick<SequentialThinkingNode, 'displayLabel'>
  | Pick<ToolNode, 'displayLabel'>
  | Record<string, never>;

export type { PipelineNode, PipelineNodeType };
