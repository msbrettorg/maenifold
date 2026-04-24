// Public surface: createNode and createNodeId. Type aliases and helpers
// live in pipeline-node-factory-types.ts and pipeline-node-factory-helpers.ts
// so this file can stay focused on the dispatch logic.
import type {
  PipelineNode,
  PipelineNodeType,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';
import {
  buildDisplayLabel,
  buildExecutionFields,
  buildNodeAdopt,
  buildWorkflowExecutionFields,
  DEFAULT_TOTAL_THOUGHTS,
  normalizeOptionalText,
} from './pipeline-node-factory-helpers';
import type {
  PipelineNodeInput,
  PipelineNodeInputByType,
  PipelineNodeByType,
  PromptNodeInput,
  SequentialThinkingNodeInput,
  ToolNodeInput,
  WorkflowNodeInput,
} from './pipeline-node-factory-types';

export { normalizeNodeCwd } from './pipeline-types';
export type {
  PipelineNodeInput,
  PipelineNodeInputByType,
  PipelineNodeByType,
  PipelineNodeDraftByType,
  PromptNodeInput,
  SequentialThinkingNodeInput,
  ToolNodeInput,
  WorkflowNodeInput,
  PipelineNodeExecutionFields,
  WorkflowNodeExecutionSelection,
} from './pipeline-node-factory-types';

export function createNodeId(type: PipelineNodeType): string {
  return typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? `${type}-${crypto.randomUUID()}`
    : `${type}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function createNode(type: 'workflow', fields: WorkflowNodeInput): WorkflowNode;
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
      const f = fields as WorkflowNodeInput;
      if (typeof f.workflowId !== 'string' || f.workflowId.length === 0) {
        throw new Error('Workflow nodes require workflowId');
      }
      const adopt = buildNodeAdopt(f.adopt);
      const requiresRole = normalizeOptionalText(f.requiresRole, 'requiresRole');
      return {
        id: f.id,
        type: 'workflow',
        ...buildDisplayLabel(f.displayLabel),
        workflowId: f.workflowId,
        ...((f.acpClient ?? '').length > 0 ? { acpClient: f.acpClient } : {}),
        ...(adopt ? { adopt } : {}),
        ...(requiresRole ? { requiresRole } : {}),
        ...(typeof f.prompt === 'string' && f.prompt.length > 0 ? { prompt: f.prompt } : {}),
        ...buildWorkflowExecutionFields(f),
      };
    }
    case 'prompt': {
      const f = fields as PromptNodeInput;
      if (typeof f.prompt !== 'string' || f.prompt.length === 0) {
        throw new Error('Prompt nodes require prompt');
      }
      if (typeof f.agent !== 'string' || f.agent.length === 0) {
        throw new Error('Prompt nodes require agent');
      }
      if (f.sessionRef) {
        if (typeof f.sessionRef.agent !== 'string' || f.sessionRef.agent.length === 0) {
          throw new Error('Prompt node sessionRef requires agent');
        }
        if (
          typeof f.sessionRef.sessionId !== 'string'
          || f.sessionRef.sessionId.length === 0
        ) {
          throw new Error('Prompt node sessionRef requires sessionId');
        }
        if (f.sessionRef.agent !== f.agent) {
          throw new Error('Prompt node sessionRef.agent must match agent');
        }
      }
      const adopt = buildNodeAdopt(f.adopt);
      const requiresRole = normalizeOptionalText(f.requiresRole, 'requiresRole');
      return {
        id: f.id,
        type: 'prompt',
        ...buildDisplayLabel(f.displayLabel),
        prompt: f.prompt,
        agent: f.agent,
        sessionRef: f.sessionRef
          ? { agent: f.sessionRef.agent, sessionId: f.sessionRef.sessionId }
          : undefined,
        ...(adopt ? { adopt } : {}),
        ...(requiresRole ? { requiresRole } : {}),
        ...(typeof f.toolMode === 'string' && f.toolMode.length > 0
          ? { toolMode: f.toolMode }
          : {}),
        ...buildExecutionFields(f),
      };
    }
    case 'sequentialThinking': {
      const f = fields as SequentialThinkingNodeInput;
      if (typeof f.topic !== 'string' || f.topic.length === 0) {
        throw new Error('Sequential thinking nodes require topic');
      }
      if (typeof f.agent !== 'string' || f.agent.length === 0) {
        throw new Error('Sequential thinking nodes require agent');
      }
      const adopt = buildNodeAdopt(f.adopt);
      const requiresRole = normalizeOptionalText(f.requiresRole, 'requiresRole');
      return {
        id: f.id,
        type: 'sequentialThinking',
        ...buildDisplayLabel(f.displayLabel),
        topic: f.topic,
        agent: f.agent,
        ...(adopt ? { adopt } : {}),
        ...(requiresRole ? { requiresRole } : {}),
        totalThoughts:
          typeof f.totalThoughts === 'number' && Number.isFinite(f.totalThoughts)
            ? f.totalThoughts
            : DEFAULT_TOTAL_THOUGHTS,
        ...(typeof f.analysisType === 'string' && f.analysisType.length > 0
          ? { analysisType: f.analysisType }
          : {}),
        ...buildExecutionFields(f),
      };
    }
    case 'tool': {
      const f = fields as ToolNodeInput;
      if (typeof f.toolName !== 'string' || f.toolName.length === 0) {
        throw new Error('Tool nodes require toolName');
      }
      if (
        f.toolArgs !== undefined
        && (typeof f.toolArgs !== 'object' || f.toolArgs === null || Array.isArray(f.toolArgs))
      ) {
        throw new Error('Tool nodes require toolArgs to be a JSON object');
      }
      return {
        id: f.id,
        type: 'tool',
        ...buildDisplayLabel(f.displayLabel),
        toolName: f.toolName,
        ...(typeof f.agent === 'string' && f.agent.length > 0 ? { agent: f.agent } : {}),
        ...(f.toolArgs ? { toolArgs: f.toolArgs } : {}),
        ...buildExecutionFields(f),
      };
    }
    default:
      throw new Error(`Unknown pipeline node type: ${type}`);
  }
}
