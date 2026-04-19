// Draft builders & validators for pipeline node form. Extracted from
// pipeline-node-form-helpers.ts to keep modules under 300 LOC.
import type {
  PipelineNodeDraftByType,
  PipelineNodeType,
} from '../lib/pipeline-types';
import { normalizeNodeCwd } from '../lib/pipeline-node-factory';
import { normalizePipelineNodeAgentConfig } from './pipeline-node-form-agent-config';
import type { PipelineNodeFormValues } from './pipeline-node-form-types';

const trim = (value: string) => value.trim();

function normalizeToolArgsRecord(
  value: Record<string, unknown>,
): Record<string, unknown> | undefined {
  return Object.keys(value).length > 0 ? value : undefined;
}

export function parsePipelineToolArgs(
  value: Record<string, unknown> | string,
): Record<string, unknown> | undefined | null {
  if (typeof value !== 'string') {
    return normalizeToolArgsRecord(value);
  }

  const raw = trim(value);
  if (!raw) return undefined;

  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

export function canSubmitPipelineNodeForm(
  nodeType: PipelineNodeType,
  values: PipelineNodeFormValues,
): boolean {
  switch (nodeType) {
    case 'prompt':
      return Boolean(trim(values.promptAgent) && trim(values.promptTemplate));
    case 'sequentialThinking':
      return Boolean(trim(values.thinkingTopic) && trim(values.thinkingAgent));
    case 'tool':
      return Boolean(
        trim(values.toolName)
        && values.toolArgsValid
        && parsePipelineToolArgs(values.toolArgs) !== null,
      );
    default:
      return Boolean(trim(values.workflowId));
  }
}

export function buildWorkflowDraft(
  values: PipelineNodeFormValues,
): PipelineNodeDraftByType['workflow'] {
  return {
    workflowId: trim(values.workflowId),
    ...(trim(values.workflowAgent) ? { acpClient: trim(values.workflowAgent) } : {}),
    ...(trim(values.workflowPrompt) ? { prompt: trim(values.workflowPrompt) } : {}),
    cwd: normalizeNodeCwd(values.cwd),
    ...(normalizePipelineNodeAgentConfig(values.workflowAgentConfig)
      ? { agentConfig: normalizePipelineNodeAgentConfig(values.workflowAgentConfig) }
      : {}),
  };
}

export function buildPipelineNodeDraft(
  nodeType: 'workflow',
  values: PipelineNodeFormValues,
): PipelineNodeDraftByType['workflow'];
export function buildPipelineNodeDraft(
  nodeType: 'prompt',
  values: PipelineNodeFormValues,
): PipelineNodeDraftByType['prompt'];
export function buildPipelineNodeDraft(
  nodeType: 'sequentialThinking',
  values: PipelineNodeFormValues,
): PipelineNodeDraftByType['sequentialThinking'];
export function buildPipelineNodeDraft(
  nodeType: 'tool',
  values: PipelineNodeFormValues,
): PipelineNodeDraftByType['tool'];
export function buildPipelineNodeDraft(
  nodeType: PipelineNodeType,
  values: PipelineNodeFormValues,
): PipelineNodeDraftByType[PipelineNodeType] {
  switch (nodeType) {
    case 'prompt':
      return {
        agent: trim(values.promptAgent),
        prompt: trim(values.promptTemplate),
        cwd: normalizeNodeCwd(values.cwd),
        ...(normalizePipelineNodeAgentConfig(values.promptAgentConfig)
          ? { agentConfig: normalizePipelineNodeAgentConfig(values.promptAgentConfig) }
          : {}),
      };
    case 'sequentialThinking':
      return {
        topic: trim(values.thinkingTopic),
        agent: trim(values.thinkingAgent),
        ...(trim(values.thinkingTotalThoughts)
          ? { totalThoughts: Number(values.thinkingTotalThoughts) }
          : {}),
        ...(trim(values.thinkingAnalysisType)
          ? { analysisType: trim(values.thinkingAnalysisType) }
          : {}),
        cwd: normalizeNodeCwd(values.cwd),
        ...(normalizePipelineNodeAgentConfig(values.thinkingAgentConfig)
          ? { agentConfig: normalizePipelineNodeAgentConfig(values.thinkingAgentConfig) }
          : {}),
      };
    case 'tool': {
      const parsedToolArgs = parsePipelineToolArgs(values.toolArgs);
      const normalizedToolArgs =
        parsedToolArgs && typeof parsedToolArgs === 'object'
          ? normalizeToolArgsRecord(parsedToolArgs)
          : parsedToolArgs;
      return {
        toolName: trim(values.toolName),
        ...(trim(values.toolAgent) ? { agent: trim(values.toolAgent) } : {}),
        cwd: normalizeNodeCwd(values.cwd),
        ...(normalizePipelineNodeAgentConfig(values.toolAgentConfig)
          ? { agentConfig: normalizePipelineNodeAgentConfig(values.toolAgentConfig) }
          : {}),
        ...(normalizedToolArgs ? { toolArgs: normalizedToolArgs } : {}),
      };
    }
    default:
      return buildWorkflowDraft(values);
  }
}
