import type {
  PipelineNode,
  PipelineNodeAdopt,
  PipelineNodeAgentConfig,
  PipelineNodeType,
  SessionRef,
} from './pipeline-types-node';
import type { ToolProvenance, ViewNode } from './pipeline-types-view';
import {
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
} from './pipeline-types-node';
import { createNode } from './pipeline-node-factory';

export type FlowNodeData = {
  nodeId: string;
  displayLabel: string;
  type: PipelineNodeType;
  model?: string;
  agentConfig?: PipelineNodeAgentConfig;
  acpSessionId?: string;
  thinkingSessionId?: string;
  workflowId?: string;
  acpClient?: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  prompt?: string;
  agent?: string;
  sessionRef?: SessionRef;
  topic?: string;
  totalThoughts?: number;
  thinkingTraceLength?: number;
  analysisType?: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  toolProvenance?: ToolProvenance | null;
  status?: ViewNode['status'];
  attempts?: number;
  failureReason?: string | null;
  reconnected?: boolean;
};

function getNodeModel(node: PipelineNode): string | undefined {
  const fromAgentConfig = 'agentConfig' in node ? node.agentConfig?.model : undefined;
  return fromAgentConfig ?? ('model' in node ? node.model : undefined);
}

function cloneAgentConfig(
  agentConfig?: PipelineNodeAgentConfig,
): PipelineNodeAgentConfig | undefined {
  if (!agentConfig) {
    return undefined;
  }
  return {
    ...agentConfig,
    ...(agentConfig.configOptions
      ? { configOptions: { ...agentConfig.configOptions } }
      : {}),
  };
}

function cloneSessionRef(sessionRef?: SessionRef): SessionRef | undefined {
  return sessionRef
    ? { agent: sessionRef.agent, sessionId: sessionRef.sessionId }
    : undefined;
}

function cloneToolProvenance(
  provenance?: ToolProvenance | null,
): ToolProvenance | null | undefined {
  if (!provenance) {
    return provenance;
  }
  return {
    toolName: provenance.toolName,
    ...(provenance.resolvedArgs
      ? { resolvedArgs: { ...provenance.resolvedArgs } }
      : {}),
    ...(provenance.responseSummary != null
      ? { responseSummary: provenance.responseSummary }
      : {}),
  };
}

function displayLabelFields(nodeId: string, displayLabel?: string): { displayLabel?: string } {
  return displayLabel && displayLabel !== nodeId ? { displayLabel } : {};
}

function optionalModel(model?: string): { model?: string } {
  return model ? { model } : {};
}

function mergeWorkflowAgentConfig(node: ViewNode): { agentConfig?: PipelineNodeAgentConfig } {
  const effectiveModel =
    (node.agentConfig?.model?.length ? node.agentConfig.model : undefined) ??
    (node.model?.length ? node.model : undefined);
  const effectiveMode = node.agentConfig?.mode?.length ? node.agentConfig.mode : undefined;
  const effectiveThoughtLevel =
    (node.agentConfig?.thoughtLevel?.length
      ? node.agentConfig.thoughtLevel
      : undefined) ?? (node.thoughtLevel?.length ? node.thoughtLevel : undefined);
  const effectiveConfigOptions = node.agentConfig?.configOptions;
  const merged: PipelineNodeAgentConfig = {
    ...(effectiveModel ? { model: effectiveModel } : {}),
    ...(effectiveMode ? { mode: effectiveMode } : {}),
    ...(effectiveThoughtLevel ? { thoughtLevel: effectiveThoughtLevel } : {}),
    ...(effectiveConfigOptions && Object.keys(effectiveConfigOptions).length > 0
      ? { configOptions: { ...effectiveConfigOptions } }
      : {}),
  };
  return Object.keys(merged).length > 0 ? { agentConfig: merged } : {};
}

export function toFlowNodeData(
  node: PipelineNode,
  viewNode?: ViewNode,
): FlowNodeData {
  const model = getNodeModel(node);
  const base = {
    nodeId: node.id,
    displayLabel: viewNode?.displayLabel ?? node.displayLabel ?? node.id,
    type: node.type ?? 'workflow',
    ...optionalModel(model),
    acpSessionId: viewNode?.acpSessionId,
    thinkingSessionId: viewNode?.thinkingSessionId,
    status: viewNode?.status,
    attempts: viewNode?.attempts,
    failureReason: viewNode?.failureReason,
    reconnected: viewNode?.reconnected,
  };

  if (isWorkflowNode(node)) {
    return {
      ...base,
      workflowId: node.workflowId,
      ...(node.acpClient ? { acpClient: node.acpClient } : {}),
      ...(node.adopt ? { adopt: { ...node.adopt } } : {}),
      ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
      ...(node.prompt ? { prompt: node.prompt } : {}),
      ...(node.agentConfig ? { agentConfig: cloneAgentConfig(node.agentConfig) } : {}),
    };
  }
  if (node.type === 'prompt') {
    return {
      ...base,
      agent: node.agent,
      prompt: node.prompt,
      ...(node.sessionRef ? { sessionRef: cloneSessionRef(node.sessionRef) } : {}),
      ...(node.adopt ? { adopt: { ...node.adopt } } : {}),
      ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
    };
  }
  if (isSequentialThinkingNode(node)) {
    return {
      ...base,
      topic: node.topic,
      agent: node.agent,
      ...(node.adopt ? { adopt: { ...node.adopt } } : {}),
      ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
      ...(typeof node.totalThoughts === 'number'
        ? { totalThoughts: node.totalThoughts }
        : {}),
      ...(viewNode?.type === 'sequentialThinking'
        ? { thinkingTraceLength: viewNode.thinking_trace?.length ?? 0 }
        : {}),
      ...(node.analysisType ? { analysisType: node.analysisType } : {}),
    };
  }
  if (isToolNode(node)) {
    return {
      ...base,
      ...(node.agent ? { agent: node.agent } : {}),
      toolName: node.toolName,
      ...(node.toolArgs ? { toolArgs: node.toolArgs } : {}),
      ...(viewNode?.type === 'tool' && viewNode.toolProvenance
        ? { toolProvenance: cloneToolProvenance(viewNode.toolProvenance) }
        : {}),
    };
  }
  const unreachable: never = node;
  throw new Error(`Unsupported pipeline node type: ${JSON.stringify(unreachable)}`);
}

export function toPipelineNode(data: FlowNodeData): PipelineNode {
  switch (data.type) {
    case 'prompt':
      return createNode('prompt', {
        id: data.nodeId,
        ...displayLabelFields(data.nodeId, data.displayLabel),
        prompt: data.prompt ?? '',
        agent: data.agent ?? '',
        ...(data.sessionRef ? { sessionRef: cloneSessionRef(data.sessionRef) } : {}),
        ...(data.adopt ? { adopt: { ...data.adopt } } : {}),
        ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
        ...optionalModel(data.model),
      });
    case 'sequentialThinking':
      return createNode('sequentialThinking', {
        id: data.nodeId,
        ...displayLabelFields(data.nodeId, data.displayLabel),
        topic: data.topic ?? '',
        agent: data.agent ?? '',
        ...(data.adopt ? { adopt: { ...data.adopt } } : {}),
        ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
        ...optionalModel(data.model),
        ...(typeof data.totalThoughts === 'number'
          ? { totalThoughts: data.totalThoughts }
          : {}),
        ...(data.analysisType ? { analysisType: data.analysisType } : {}),
      });
    case 'tool':
      return createNode('tool', {
        id: data.nodeId,
        ...displayLabelFields(data.nodeId, data.displayLabel),
        toolName: data.toolName ?? '',
        ...(data.agent ? { agent: data.agent } : {}),
        ...optionalModel(data.model),
        ...(data.toolArgs ? { toolArgs: data.toolArgs } : {}),
      });
    default:
      return createNode('workflow', {
        id: data.nodeId,
        ...displayLabelFields(data.nodeId, data.displayLabel),
        workflowId: data.workflowId ?? '',
        acpClient: data.acpClient ?? '',
        ...(data.adopt ? { adopt: { ...data.adopt } } : {}),
        ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
        ...optionalModel(data.model),
        ...(data.agentConfig ? { agentConfig: data.agentConfig } : {}),
        ...(data.prompt ? { prompt: data.prompt } : {}),
      });
  }
}

export function toPipelineNodeFromViewNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt':
      return createNode('prompt', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.sessionRef ? { sessionRef: cloneSessionRef(node.sessionRef) } : {}),
        ...(node.adopt ? { adopt: { ...node.adopt } } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...optionalModel(node.agentConfig?.model ?? node.model),
      });
    case 'sequentialThinking':
      return createNode('sequentialThinking', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(node.adopt ? { adopt: { ...node.adopt } } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...optionalModel(node.agentConfig?.model ?? node.model),
        ...(typeof node.totalThoughts === 'number'
          ? { totalThoughts: node.totalThoughts }
          : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
      });
    case 'tool':
      return createNode('tool', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...optionalModel(node.agentConfig?.model ?? node.model),
        ...(node.toolArgs ? { toolArgs: node.toolArgs } : {}),
      });
    default:
      return createNode('workflow', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        acpClient: node.acpClient ?? '',
        ...(node.adopt ? { adopt: { ...node.adopt } } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...mergeWorkflowAgentConfig(node),
        ...(node.prompt ? { prompt: node.prompt } : {}),
      });
  }
}
