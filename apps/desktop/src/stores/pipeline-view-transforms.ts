import type {
  PipelineDefinition,
  PipelineListItem,
  PipelineNodeAgentConfig,
  PipelineView,
  PipelineNode,
  ViewNode,
} from '../lib/pipeline-types';
interface PipelineListResult {
  pipelines?: PipelineListItem[];
}

interface PipelineCreateResult {
  pipelineId?: string;
}

interface LegacyExecutionFields {
  model?: string;
  thoughtLevel?: string;
  agentConfig?: PipelineNodeAgentConfig;
}

export function normalizePipelineList(
  result: PipelineListItem[] | PipelineListResult | undefined,
): PipelineListItem[] {
  return Array.isArray(result) ? result : result?.pipelines ?? [];
}

export function isCreateResult(value: unknown): value is PipelineCreateResult {
  return typeof value === 'object' && value !== null && 'pipelineId' in value;
}

function normalizeViewNode(node: ViewNode): ViewNode {
  const normalized = node.type === undefined ? { ...node, type: 'workflow' as const } : { ...node };

  if ('toolArgs' in normalized && normalized.toolArgs) {
    normalized.toolArgs = { ...normalized.toolArgs };
  }

  if ('allowedTools' in normalized && Array.isArray(normalized.allowedTools)) {
    normalized.allowedTools = [...normalized.allowedTools];
  }

  if ('agentConfig' in normalized && normalized.agentConfig) {
    normalized.agentConfig = cloneAgentConfig(normalized.agentConfig);
  }

  return normalized;
}

function cloneAgentConfig(
  agentConfig: PipelineNodeAgentConfig,
): PipelineNodeAgentConfig {
  return {
    ...(agentConfig.model ? { model: agentConfig.model } : {}),
    ...(agentConfig.mode ? { mode: agentConfig.mode } : {}),
    ...(agentConfig.thoughtLevel ? { thoughtLevel: agentConfig.thoughtLevel } : {}),
    ...(agentConfig.configOptions
      ? { configOptions: { ...agentConfig.configOptions } }
      : {}),
  };
}

function firstNonEmptyString(...values: Array<string | undefined>): string | undefined {
  return values.find((value): value is string => typeof value === 'string' && value.length > 0);
}

function mergeAgentConfig(node: LegacyExecutionFields): PipelineNodeAgentConfig | undefined {
  const configOptions = node.agentConfig?.configOptions
    ? { ...node.agentConfig.configOptions }
    : undefined;

  const merged: PipelineNodeAgentConfig = {
    ...(firstNonEmptyString(node.agentConfig?.model, node.model)
      ? { model: firstNonEmptyString(node.agentConfig?.model, node.model) }
      : {}),
    ...(firstNonEmptyString(node.agentConfig?.mode)
      ? { mode: firstNonEmptyString(node.agentConfig?.mode) }
      : {}),
    ...(firstNonEmptyString(node.agentConfig?.thoughtLevel, node.thoughtLevel)
      ? { thoughtLevel: firstNonEmptyString(node.agentConfig?.thoughtLevel, node.thoughtLevel) }
      : {}),
    ...(configOptions && Object.keys(configOptions).length > 0
      ? { configOptions }
      : {}),
  };

  return Object.keys(merged).length > 0 ? merged : undefined;
}

export function normalizeView(view: PipelineView): PipelineView {
  return {
    ...view,
    nodes: view.nodes.map(normalizeViewNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}

function toLocalNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt': {
      const agentConfig = mergeAgentConfig(node);
      return {
        id: node.nodeId,
        type: 'prompt',
        prompt: node.prompt,
        agent: node.agent,
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'sequentialThinking': {
      const agentConfig = mergeAgentConfig(node);
      return {
        id: node.nodeId,
        type: 'sequentialThinking',
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'workflow':
    case undefined:
      return {
        id: node.nodeId,
        type: 'workflow',
        workflowId: node.workflowId,
        ...(node.acpClient ? { acpClient: node.acpClient } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
      };
    case 'tool': {
      const agentConfig = mergeAgentConfig(node);
      return {
        id: node.nodeId,
        type: 'tool',
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
  }
}

export function toLocalDefinition(view: PipelineView): PipelineDefinition {
  return {
    nodes: view.nodes.map(toLocalNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}

function serializeNode(node: PipelineNode): PipelineNode {
  switch (node.type) {
    case 'prompt': {
      const agentConfig = mergeAgentConfig(node);
      return {
        id: node.id,
        type: 'prompt',
        prompt: node.prompt,
        agent: node.agent,
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'sequentialThinking': {
      const agentConfig = mergeAgentConfig(node);
      return {
        id: node.id,
        type: 'sequentialThinking',
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'workflow':
    case undefined:
      return {
        id: node.id,
        type: 'workflow',
        workflowId: node.workflowId,
        ...(node.acpClient ? { acpClient: node.acpClient } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
      };
    case 'tool': {
      const agentConfig = mergeAgentConfig(node);
      return {
        id: node.id,
        type: 'tool',
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
  }
}

export function serializePipelineDefinition(definition: PipelineDefinition): PipelineDefinition {
  return {
    nodes: definition.nodes.map(serializeNode),
    edges: definition.edges.map((edge) => ({ ...edge })),
  };
}
