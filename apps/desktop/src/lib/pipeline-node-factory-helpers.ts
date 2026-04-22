// Pure helpers (clone/normalize/build) used by createNode in pipeline-node-factory.
// Extracted to keep pipeline-node-factory.ts under 300 LOC.
import type { PipelineNodeAdopt, PipelineNodeAgentConfig } from './pipeline-types';
import { cwdHasReasonablePathPrefix, normalizeNodeCwd } from './pipeline-types';
import type {
  AnyDisplayLabelPick,
  PipelineNodeExecutionFields,
  WorkflowNodeExecutionSelection,
} from './pipeline-node-factory-types';

export const DEFAULT_TOTAL_THOUGHTS = 5;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function cloneStringArray(
  value: string[] | undefined,
  fieldName: string,
): string[] | undefined {
  if (value === undefined) return undefined;

  if (!Array.isArray(value) || value.some((entry) => typeof entry !== 'string')) {
    throw new Error(`${fieldName} must be an array of strings`);
  }
  return [...value];
}

export function cloneOptionalNumber(
  value: number | undefined,
  fieldName: string,
): number | undefined {
  if (value === undefined) return undefined;

  if (!Number.isFinite(value) || value < 0) {
    throw new Error(`${fieldName} must be a non-negative number`);
  }
  return value;
}

export function cloneAgentConfig(
  value: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig | undefined {
  if (value === undefined) return undefined;

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
    ...(typeof value.model === 'string' && value.model.length > 0 ? { model: value.model } : {}),
    ...(typeof value.mode === 'string' && value.mode.length > 0 ? { mode: value.mode } : {}),
    ...(typeof value.thoughtLevel === 'string' && value.thoughtLevel.length > 0
      ? { thoughtLevel: value.thoughtLevel }
      : {}),
    ...(value.configOptions && Object.keys(value.configOptions).length > 0
      ? { configOptions: { ...value.configOptions } }
      : {}),
  };
}

export function normalizeExecutionCwd(value: unknown): string | undefined {
  const cwd = normalizeNodeCwd(value);
  if (cwd && !cwdHasReasonablePathPrefix(cwd)) {
    console.warn( // audit-ok: no-console
      `[pipeline-node-factory] cwd "${cwd}" does not start with a typical path prefix`,
    );
  }
  return cwd;
}

export function normalizeOptionalText(value: unknown, fieldName: string): string | undefined {
  if (value == null) return undefined;
  if (typeof value !== 'string') {
    throw new Error(`${fieldName} must be a string`);
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

export function buildNodeAdopt(
  value: PipelineNodeAdopt | undefined,
): PipelineNodeAdopt | undefined {
  if (value === undefined) return undefined;
  if (!isRecord(value)) {
    throw new Error('adopt must be a JSON object');
  }

  const adopt: PipelineNodeAdopt = {
    ...(normalizeOptionalText(value.role, 'adopt.role')
      ? { role: normalizeOptionalText(value.role, 'adopt.role') }
      : {}),
    ...(normalizeOptionalText(value.color, 'adopt.color')
      ? { color: normalizeOptionalText(value.color, 'adopt.color') }
      : {}),
    ...(normalizeOptionalText(value.perspective, 'adopt.perspective')
      ? { perspective: normalizeOptionalText(value.perspective, 'adopt.perspective') }
      : {}),
  };

  return Object.keys(adopt).length > 0 ? adopt : undefined;
}

export function buildExecutionFields(
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

export function buildWorkflowExecutionFields(
  fields: WorkflowNodeExecutionSelection,
): WorkflowNodeExecutionSelection {
  const cwd = normalizeExecutionCwd(fields.cwd);
  const timeout = cloneOptionalNumber(fields.timeout, 'timeout');
  const agentConfig = cloneAgentConfig(fields.agentConfig);

  return {
    ...(typeof fields.model === 'string' && fields.model.length > 0
      ? { model: fields.model }
      : {}),
    ...(typeof fields.thoughtLevel === 'string' && fields.thoughtLevel.length > 0
      ? { thoughtLevel: fields.thoughtLevel }
      : {}),
    ...(cwd ? { cwd } : {}),
    ...(timeout !== undefined ? { timeout } : {}),
    ...(agentConfig ? { agentConfig } : {}),
  };
}

export function buildDisplayLabel(value: string | undefined): AnyDisplayLabelPick {
  return typeof value === 'string' && value.length > 0
    ? { displayLabel: value }
    : {};
}
