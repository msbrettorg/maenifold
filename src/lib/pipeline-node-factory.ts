import type {
  PipelineNode,
  PipelineNodeType,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';

export interface WorkflowNodeInput {
  id: string;
  workflowId: string;
  agent: string;
  prompt?: string;
}

export interface PromptNodeInput {
  id: string;
  prompt: string;
  agent: string;
}

export interface SequentialThinkingNodeInput {
  id: string;
  topic: string;
  agent: string;
  totalThoughts?: number;
  analysisType?: string;
}

export interface ToolNodeInput {
  id: string;
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

const DEFAULT_TOTAL_THOUGHTS = 5;

export function createNodeId(type: PipelineNodeType): string {
  return typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? `${type}-${crypto.randomUUID()}`
    : `${type}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function createNode(
  type: 'workflow',
  fields: WorkflowNodeInput,
): WorkflowNode;
export function createNode(type: 'prompt', fields: PromptNodeInput): PromptNode;
export function createNode(
  type: 'sequentialThinking',
  fields: SequentialThinkingNodeInput,
): SequentialThinkingNode;
export function createNode(type: 'tool', fields: ToolNodeInput): ToolNode;
export function createNode<T extends PipelineNodeType>(
  type: T,
  fields: PipelineNodeInputByType[T],
): PipelineNodeByType[T];
export function createNode(
  type: PipelineNodeType,
  fields: PipelineNodeInput,
): PipelineNode {
  if (typeof fields.id !== 'string' || fields.id.length === 0) {
    throw new Error(`${type} nodes require id`);
  }

  switch (type) {
    case 'workflow': {
      const workflowFields = fields as WorkflowNodeInput;

      if (
        typeof workflowFields.workflowId !== 'string' ||
        workflowFields.workflowId.length === 0
      ) {
        throw new Error('Workflow nodes require workflowId');
      }

      return {
        id: workflowFields.id,
        type: 'workflow',
        workflowId: workflowFields.workflowId,
        ...(workflowFields.agent.length > 0
          ? { acpClient: workflowFields.agent }
          : {}),
        ...(typeof workflowFields.prompt === 'string' &&
        workflowFields.prompt.length > 0
          ? { prompt: workflowFields.prompt }
          : {}),
      };
    }
    case 'prompt': {
      const promptFields = fields as PromptNodeInput;

      if (
        typeof promptFields.prompt !== 'string' ||
        promptFields.prompt.length === 0
      ) {
        throw new Error('Prompt nodes require prompt');
      }
      if (
        typeof promptFields.agent !== 'string' ||
        promptFields.agent.length === 0
      ) {
        throw new Error('Prompt nodes require agent');
      }

      return {
        id: promptFields.id,
        type: 'prompt',
        prompt: promptFields.prompt,
        agent: promptFields.agent,
      };
    }
    case 'sequentialThinking': {
      const sequentialThinkingFields = fields as SequentialThinkingNodeInput;

      if (
        typeof sequentialThinkingFields.topic !== 'string' ||
        sequentialThinkingFields.topic.length === 0
      ) {
        throw new Error('Sequential thinking nodes require topic');
      }
      if (
        typeof sequentialThinkingFields.agent !== 'string' ||
        sequentialThinkingFields.agent.length === 0
      ) {
        throw new Error('Sequential thinking nodes require agent');
      }

      return {
        id: sequentialThinkingFields.id,
        type: 'sequentialThinking',
        topic: sequentialThinkingFields.topic,
        agent: sequentialThinkingFields.agent,
        totalThoughts:
          typeof sequentialThinkingFields.totalThoughts === 'number' &&
          Number.isFinite(sequentialThinkingFields.totalThoughts)
            ? sequentialThinkingFields.totalThoughts
            : DEFAULT_TOTAL_THOUGHTS,
        ...(typeof sequentialThinkingFields.analysisType === 'string' &&
        sequentialThinkingFields.analysisType.length > 0
          ? { analysisType: sequentialThinkingFields.analysisType }
          : {}),
      };
    }
    case 'tool': {
      const toolFields = fields as ToolNodeInput;

      if (
        typeof toolFields.toolName !== 'string' ||
        toolFields.toolName.length === 0
      ) {
        throw new Error('Tool nodes require toolName');
      }
      if (
        toolFields.toolArgs !== undefined &&
        (typeof toolFields.toolArgs !== 'object' ||
          toolFields.toolArgs === null ||
          Array.isArray(toolFields.toolArgs))
      ) {
        throw new Error('Tool nodes require toolArgs to be a JSON object');
      }

      return {
        id: toolFields.id,
        type: 'tool',
        toolName: toolFields.toolName,
        ...(typeof toolFields.agent === 'string' && toolFields.agent.length > 0
          ? { agent: toolFields.agent }
          : {}),
        ...(toolFields.toolArgs ? { toolArgs: toolFields.toolArgs } : {}),
      };
    }
    default:
      throw new Error(`Unknown pipeline node type: ${type}`);
  }
}
