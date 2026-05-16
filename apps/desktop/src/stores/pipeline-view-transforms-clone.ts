// Shared clone helpers + legacy field merging for pipeline view transforms.

import type {
  NodeProvenance,
  ToolProvenance,
} from '../lib/pipeline-types-view';
import type { PipelineInputs } from '../lib/pipeline-types-edge';
import type {
  PipelineNodeAdopt,
  PipelineNodeAgentConfig,
  SessionRef,
} from '../lib/pipeline-types-node';

export interface LegacyExecutionFields {
  model?: string;
  thoughtLevel?: string;
  agentConfig?: PipelineNodeAgentConfig;
}

export function cloneNodeAdopt(adopt: PipelineNodeAdopt): PipelineNodeAdopt {
  return {
    ...(adopt.role ? { role: adopt.role } : {}),
    ...(adopt.color ? { color: adopt.color } : {}),
    ...(adopt.perspective ? { perspective: adopt.perspective } : {}),
  };
}

export function cloneAgentConfig(
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

export function cloneToolProvenance(toolProvenance: ToolProvenance): ToolProvenance {
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

export function cloneSessionRef(sessionRef: SessionRef): SessionRef {
  return {
    agent: sessionRef.agent,
    sessionId: sessionRef.sessionId,
  };
}

export function cloneNodeProvenance(provenance: NodeProvenance): NodeProvenance {
  return {
    ...(provenance.agent !== undefined ? { agent: provenance.agent } : {}),
    inspectable: provenance.inspectable,
    detail: {
      ...(provenance.detail.sessionId !== undefined ? { sessionId: provenance.detail.sessionId } : {}),
      ...(provenance.detail.thinkingSessionId !== undefined ? { thinkingSessionId: provenance.detail.thinkingSessionId } : {}),
      ...(provenance.detail.parentWorkflowId !== undefined ? { parentWorkflowId: provenance.detail.parentWorkflowId } : {}),
      ...(provenance.detail.lineageGateTaskId !== undefined ? { lineageGateTaskId: provenance.detail.lineageGateTaskId } : {}),
      ...(provenance.detail.lineagePipelineRunId !== undefined ? { lineagePipelineRunId: provenance.detail.lineagePipelineRunId } : {}),
      ...(provenance.detail.lineageThinkingSessionId !== undefined ? { lineageThinkingSessionId: provenance.detail.lineageThinkingSessionId } : {}),
      ...(provenance.detail.lineageMemoryNoteId !== undefined ? { lineageMemoryNoteId: provenance.detail.lineageMemoryNoteId } : {}),
      ...(provenance.detail.parent_workflow_id !== undefined ? { parent_workflow_id: provenance.detail.parent_workflow_id } : {}),
      ...(provenance.detail.lineage_gate_task_id !== undefined ? { lineage_gate_task_id: provenance.detail.lineage_gate_task_id } : {}),
      ...(provenance.detail.lineage_pipeline_run_id !== undefined ? { lineage_pipeline_run_id: provenance.detail.lineage_pipeline_run_id } : {}),
      ...(provenance.detail.lineage_thinking_session_id !== undefined ? { lineage_thinking_session_id: provenance.detail.lineage_thinking_session_id } : {}),
      ...(provenance.detail.lineage_memory_note_id !== undefined ? { lineage_memory_note_id: provenance.detail.lineage_memory_note_id } : {}),
      ...(provenance.detail.adoption
        ? {
            adoption: {
              role: provenance.detail.adoption.role,
              color: provenance.detail.adoption.color,
              perspective: provenance.detail.adoption.perspective,
            },
          }
        : {}),
    },
  };
}

export function cloneInputs(inputs: PipelineInputs | undefined): PipelineInputs | undefined {
  if (!inputs) {
    return undefined;
  }

  return { ...inputs };
}

function firstNonEmptyString(...values: Array<string | undefined>): string | undefined {
  return values.find((value): value is string => typeof value === 'string' && value.length > 0);
}

export function cloneAndExtractAgentConfig(
  agentConfig: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig | undefined {
  if (!agentConfig) return undefined;
  return cloneAgentConfig(agentConfig);
}

// COMPAT: remove when backend drops top-level model/thoughtLevel from wire format
export function mergeLegacyExecutionFields(
  node: LegacyExecutionFields,
): PipelineNodeAgentConfig | undefined {
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
    ...(configOptions && Object.keys(configOptions).length > 0 ? { configOptions } : {}),
  };

  return Object.keys(merged).length > 0 ? merged : undefined;
}
