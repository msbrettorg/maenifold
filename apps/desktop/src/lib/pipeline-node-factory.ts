import type {
  PipelineNodeAgentConfig,
  PipelineNode,
  PipelineNodeType,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';
import {
  cwdHasReasonablePathPrefix,
  normalizeNodeCwd,
} from './pipeline-types';

export { normalizeNodeCwd } from './pipeline-types';

interface PipelineNodeExecutionFields {
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

interface WorkflowNodeExecutionSelection {
  model?: string;
  thoughtLevel?: string;
  cwd?: string;
  agentConfig?: PipelineNodeAgentConfig;
}

export interface WorkflowNodeInput extends WorkflowNodeExecutionSelection {
  id: string;
  displayLabel?: string;
  workflowId: string;
  acpClient?: string;
  agent?: string;
  prompt?: string;
}

export interface PromptNodeInput extends PipelineNodeExecutionFields {
  id: string;
  displayLabel?: string;
  prompt: string;
  agent: string;
  toolMode?: string;
}

export interface SequentialThinkingNodeInput extends PipelineNodeExecutionFields {
  id: string;
  displayLabel?: string;
  topic: string;
  agent: string;
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

const DEFAULT_TOTAL_THOUGHTS = 5;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function cloneStringArray(value: string[] | undefined, fieldName: string): string[] | undefined {
  if (value === undefined) {
    return undefined;
  }

  if (!Array.isArray(value) || value.some((entry) => typeof entry !== 'string')) {
    throw new Error(`${fieldName} must be an array of strings`);
  }

  return [...value];
}

function cloneOptionalNumber(
  value: number | undefined,
  fieldName: string,
): number | undefined {
  if (value === undefined) {
    return undefined;
  }

  if (!Number.isFinite(value) || value < 0) {
    throw new Error(`${fieldName} must be a non-negative number`);
  }

  return value;
}

function cloneAgentConfig(
  value: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig | undefined {
  if (value === undefined) {
    return undefined;
  }

  if (!isRecord(value)) {
    throw new Error('agentConfig must be a JSON object');
  }

  if (value.model !== undefined && typeof value.model !== 'string') {
    throw new Error('agentConfig.model must be a string');
  }

  if (value.mode !== undefined && typeof value.mode !== 'string') {
    throw new Error('agentConfig.mode must be a string');
  }

  if (value.thoughtLevel !== undefined && typeof value.thoughtLevel !== 'string') {
    throw new Error('agentConfig.thoughtLevel must be a string');
  }

  if (
    value.configOptions !== undefined
    && (!isRecord(value.configOptions) || Array.isArray(value.configOptions))
  ) {
    throw new Error('agentConfig.configOptions must be a JSON object');
  }

  return {
    ...(typeof value.model === 'string' && value.model.length > 0
      ? { model: value.model }
      : {}),
    ...(typeof value.mode === 'string' && value.mode.length > 0
      ? { mode: value.mode }
      : {}),
    ...(typeof value.thoughtLevel === 'string' && value.thoughtLevel.length > 0
      ? { thoughtLevel: value.thoughtLevel }
      : {}),
    ...(value.configOptions && Object.keys(value.configOptions).length > 0
      ? { configOptions: { ...value.configOptions } }
      : {}),
  };
}

function normalizeExecutionCwd(value: unknown): string | undefined {
  const cwd = normalizeNodeCwd(value);

  if (cwd && !cwdHasReasonablePathPrefix(cwd)) {
    console.warn(
      `[pipeline-node-factory] cwd "${cwd}" does not start with a typical path prefix`,
    );
  }

  return cwd;
}

function buildExecutionFields(
  fields: PipelineNodeExecutionFields,
): PipelineNodeExecutionFields {
  const allowedTools = cloneStringArray(fields.allowedTools, 'allowedTools');
  const maxTurns = cloneOptionalNumber(fields.maxTurns, 'maxTurns');
  const timeout = cloneOptionalNumber(fields.timeout, 'timeout');
  const cwd = normalizeExecutionCwd(fields.cwd);
  const agentConfig = cloneAgentConfig(fields.agentConfig);

  return {
    ...(typeof fields.model === 'string' && fields.model.length > 0
      ? { model: fields.model }
      : {}),
    ...(typeof fields.thoughtLevel === 'string' && fields.thoughtLevel.length > 0
      ? { thoughtLevel: fields.thoughtLevel }
      : {}),
    ...(allowedTools ? { allowedTools } : {}),
    ...(maxTurns !== undefined ? { maxTurns } : {}),
    ...(cwd ? { cwd } : {}),
    ...(timeout !== undefined ? { timeout } : {}),
    ...(agentConfig ? { agentConfig } : {}),
  };
}

function buildWorkflowExecutionFields(
  fields: WorkflowNodeExecutionSelection,
): WorkflowNodeExecutionSelection {
  const cwd = normalizeExecutionCwd(fields.cwd);
  const agentConfig = cloneAgentConfig(fields.agentConfig);

  return {
    ...(typeof fields.model === 'string' && fields.model.length > 0
      ? { model: fields.model }
      : {}),
    ...(typeof fields.thoughtLevel === 'string' && fields.thoughtLevel.length > 0
      ? { thoughtLevel: fields.thoughtLevel }
      : {}),
    ...(cwd ? { cwd } : {}),
    ...(agentConfig ? { agentConfig } : {}),
  };
}

function buildDisplayLabel(
  value: string | undefined,
): Pick<WorkflowNode, 'displayLabel'>
| Pick<PromptNode, 'displayLabel'>
| Pick<SequentialThinkingNode, 'displayLabel'>
| Pick<ToolNode, 'displayLabel'>
| Record<string, never> {
  return typeof value === 'string' && value.length > 0
    ? { displayLabel: value }
    : {};
}

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
        ...buildDisplayLabel(workflowFields.displayLabel),
        workflowId: workflowFields.workflowId,
        ...((workflowFields.acpClient ?? '').length > 0
          ? { acpClient: workflowFields.acpClient }
          : {}),
        ...(typeof workflowFields.prompt === 'string' &&
        workflowFields.prompt.length > 0
          ? { prompt: workflowFields.prompt }
          : {}),
        ...buildWorkflowExecutionFields(workflowFields),
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
        ...buildDisplayLabel(promptFields.displayLabel),
        prompt: promptFields.prompt,
        agent: promptFields.agent,
        ...(typeof promptFields.toolMode === 'string' && promptFields.toolMode.length > 0
          ? { toolMode: promptFields.toolMode }
          : {}),
        ...buildExecutionFields(promptFields),
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
        ...buildDisplayLabel(sequentialThinkingFields.displayLabel),
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
        ...buildExecutionFields(sequentialThinkingFields),
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
        ...buildDisplayLabel(toolFields.displayLabel),
        toolName: toolFields.toolName,
        ...(typeof toolFields.agent === 'string' && toolFields.agent.length > 0
          ? { agent: toolFields.agent }
          : {}),
        ...(toolFields.toolArgs ? { toolArgs: toolFields.toolArgs } : {}),
        ...buildExecutionFields(toolFields),
      };
    }
    default:
      throw new Error(`Unknown pipeline node type: ${type}`);
  }
}
