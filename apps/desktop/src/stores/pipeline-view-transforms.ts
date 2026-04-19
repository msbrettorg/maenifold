import type {
  NodeProvenance,
  PipelineDefinition,
  PipelineInputs,
  PipelineListItem,
  PipelineNodeAdopt,
  PipelineNodeAgentConfig,
  PipelineView,
  PipelineNode,
  ToolProvenance,
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

function cloneNodeAdopt(adopt: PipelineNodeAdopt): PipelineNodeAdopt {
  return {
    ...(adopt.role ? { role: adopt.role } : {}),
    ...(adopt.color ? { color: adopt.color } : {}),
    ...(adopt.perspective ? { perspective: adopt.perspective } : {}),
  };
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

  if ('adopt' in normalized && normalized.adopt) {
    normalized.adopt = cloneNodeAdopt(normalized.adopt);
  }

  if ('toolProvenance' in normalized && normalized.toolProvenance) {
    normalized.toolProvenance = cloneToolProvenance(normalized.toolProvenance);
  }

  if ('provenance' in normalized && normalized.provenance) {
    normalized.provenance = cloneNodeProvenance(normalized.provenance);
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

function cloneToolProvenance(toolProvenance: ToolProvenance): ToolProvenance {
  return {
    toolName: toolProvenance.toolName,
    ...(toolProvenance.resolvedArgs
      ? { resolvedArgs: { ...toolProvenance.resolvedArgs } }
      : {}),
    ...(toolProvenance.responseSummary != null
      ? { responseSummary: toolProvenance.responseSummary }
      : {}),
  };
}

function cloneNodeProvenance(provenance: NodeProvenance): NodeProvenance {
  return {
    ...(provenance.agent !== undefined ? { agent: provenance.agent } : {}),
    inspectable: provenance.inspectable,
    detail: {
      ...(provenance.detail.sessionId !== undefined
        ? { sessionId: provenance.detail.sessionId }
        : {}),
      ...(provenance.detail.thinkingSessionId !== undefined
        ? { thinkingSessionId: provenance.detail.thinkingSessionId }
        : {}),
      ...(provenance.detail.parentWorkflowId !== undefined
        ? { parentWorkflowId: provenance.detail.parentWorkflowId }
        : {}),
      ...(provenance.detail.lineageGateTaskId !== undefined
        ? { lineageGateTaskId: provenance.detail.lineageGateTaskId }
        : {}),
      ...(provenance.detail.lineagePipelineRunId !== undefined
        ? { lineagePipelineRunId: provenance.detail.lineagePipelineRunId }
        : {}),
      ...(provenance.detail.lineageThinkingSessionId !== undefined
        ? { lineageThinkingSessionId: provenance.detail.lineageThinkingSessionId }
        : {}),
      ...(provenance.detail.lineageMemoryNoteId !== undefined
        ? { lineageMemoryNoteId: provenance.detail.lineageMemoryNoteId }
        : {}),
      ...(provenance.detail.parent_workflow_id !== undefined
        ? { parent_workflow_id: provenance.detail.parent_workflow_id }
        : {}),
      ...(provenance.detail.lineage_gate_task_id !== undefined
        ? { lineage_gate_task_id: provenance.detail.lineage_gate_task_id }
        : {}),
      ...(provenance.detail.lineage_pipeline_run_id !== undefined
        ? { lineage_pipeline_run_id: provenance.detail.lineage_pipeline_run_id }
        : {}),
      ...(provenance.detail.lineage_thinking_session_id !== undefined
        ? { lineage_thinking_session_id: provenance.detail.lineage_thinking_session_id }
        : {}),
      ...(provenance.detail.lineage_memory_note_id !== undefined
        ? { lineage_memory_note_id: provenance.detail.lineage_memory_note_id }
        : {}),
      ...(provenance.detail.adoption
        ? {
            adoption: {
              role: provenance.detail.adoption.role,
              color: provenance.detail.adoption.color,
              perspective: provenance.detail.adoption.perspective,
            },
          }
        : {}),
      canReconnect: provenance.detail.canReconnect,
    },
  };
}

function cloneInputs(inputs: PipelineInputs | undefined): PipelineInputs | undefined {
  if (!inputs) {
    return undefined;
  }

  return { ...inputs };
}

function firstNonEmptyString(...values: Array<string | undefined>): string | undefined {
  return values.find((value): value is string => typeof value === 'string' && value.length > 0);
}

function cloneAndExtractAgentConfig(
  agentConfig: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig | undefined {
  if (!agentConfig) return undefined;
  return cloneAgentConfig(agentConfig);
}

// COMPAT: remove when backend drops top-level model/thoughtLevel from wire format
function mergeLegacyExecutionFields(node: LegacyExecutionFields): PipelineNodeAgentConfig | undefined {
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
    ...(view.inputs ? { inputs: cloneInputs(view.inputs) } : {}),
    nodes: view.nodes.map(normalizeViewNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}

function toLocalNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.nodeId,
        type: 'prompt',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'sequentialThinking': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.nodeId,
        type: 'sequentialThinking',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'workflow':
    case undefined: {
      const agentConfig = cloneAndExtractAgentConfig(node.agentConfig);
      const client = node.acpClient;
      return {
        id: node.nodeId,
        type: 'workflow',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        ...(client ? { acpClient: client } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'tool': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.nodeId,
        type: 'tool',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
  }
}

export function toLocalDefinition(view: PipelineView): PipelineDefinition {
  return {
    ...(view.templateId ? { templateId: view.templateId } : {}),
    ...(view.inputs ? { inputs: cloneInputs(view.inputs) } : {}),
    nodes: view.nodes.map(toLocalNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}

function serializeNode(node: PipelineNode): PipelineNode {
  switch (node.type) {
    case 'prompt': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.id,
        type: 'prompt',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'sequentialThinking': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.id,
        type: 'sequentialThinking',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'workflow':
    case undefined: {
      const agentConfig = cloneAndExtractAgentConfig(node.agentConfig);
      return {
        id: node.id,
        type: 'workflow',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        ...(node.acpClient ? { acpClient: node.acpClient } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
    case 'tool': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.id,
        type: 'tool',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
      };
    }
  }
}

export function serializePipelineDefinition(definition: PipelineDefinition): PipelineDefinition {
  return {
    ...(definition.templateId ? { templateId: definition.templateId } : {}),
    ...(definition.inputs ? { inputs: cloneInputs(definition.inputs) } : {}),
    nodes: definition.nodes.map(serializeNode),
    edges: definition.edges.map((edge) => ({ ...edge })),
  };
}
