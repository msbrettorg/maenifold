import { ref, type Ref } from 'vue';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import type { PipelineNodeType } from '../lib/pipeline-types';

export interface PipelineNodeFormValues {
  workflowId: string;
  workflowAgent: string;
  workflowPrompt: string;
  promptAgent: string;
  promptTemplate: string;
  thinkingTopic: string;
  thinkingAgent: string;
  thinkingTotalThoughts: string;
  thinkingAnalysisType: string;
  toolName: string;
  toolArgs: Record<string, unknown>;
  toolArgsValid: boolean;
}

export const DEFAULT_PIPELINE_NODE_THOUGHTS = '5';

const trim = (value: string) => value.trim();

function normalizeToolArgsRecord(
  value: Record<string, unknown>
): Record<string, unknown> | undefined {
  return Object.keys(value).length > 0 ? value : undefined;
}

export interface PipelineNodeFormRefs {
  workflowId: Ref<string>;
  workflowAgent: Ref<string>;
  workflowPrompt: Ref<string>;
  promptAgent: Ref<string>;
  promptTemplate: Ref<string>;
  thinkingTopic: Ref<string>;
  thinkingAgent: Ref<string>;
  thinkingTotalThoughts: Ref<string>;
  thinkingAnalysisType: Ref<string>;
  toolName: Ref<string>;
  toolArgs: Ref<Record<string, unknown>>;
  toolArgsValid: Ref<boolean>;
  getFormValues: () => PipelineNodeFormValues;
}

export function usePipelineNodeFormFields(): PipelineNodeFormRefs {
  const workflowId = ref('');
  const workflowAgent = ref('');
  const workflowPrompt = ref('');
  const promptAgent = ref('');
  const promptTemplate = ref('');
  const thinkingTopic = ref('');
  const thinkingAgent = ref('');
  const thinkingTotalThoughts = ref(DEFAULT_PIPELINE_NODE_THOUGHTS);
  const thinkingAnalysisType = ref('');
  const toolName = ref('');
  const toolArgs = ref<Record<string, unknown>>({});
  const toolArgsValid = ref(true);

  return {
    workflowId,
    workflowAgent,
    workflowPrompt,
    promptAgent,
    promptTemplate,
    thinkingTopic,
    thinkingAgent,
    thinkingTotalThoughts,
    thinkingAnalysisType,
    toolName,
    toolArgs,
    toolArgsValid,
    getFormValues: () => ({
      workflowId: workflowId.value,
      workflowAgent: workflowAgent.value,
      workflowPrompt: workflowPrompt.value,
      promptAgent: promptAgent.value,
      promptTemplate: promptTemplate.value,
      thinkingTopic: thinkingTopic.value,
      thinkingAgent: thinkingAgent.value,
      thinkingTotalThoughts: thinkingTotalThoughts.value,
      thinkingAnalysisType: thinkingAnalysisType.value,
      toolName: toolName.value,
      toolArgs: toolArgs.value,
      toolArgsValid: toolArgsValid.value,
    }),
  };
}

export function parsePipelineToolArgs(
  value: Record<string, unknown> | string
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
  values: PipelineNodeFormValues
): boolean {
  switch (nodeType) {
    case 'prompt':
      return Boolean(trim(values.promptAgent) && trim(values.promptTemplate));
    case 'sequentialThinking':
      return Boolean(trim(values.thinkingTopic) && trim(values.thinkingAgent));
    case 'tool':
      return Boolean(
        trim(values.toolName) &&
        values.toolArgsValid &&
        parsePipelineToolArgs(values.toolArgs) !== null
      );
    default:
      return Boolean(trim(values.workflowId));
  }
}

export function buildPipelineNodeDraft(
  nodeType: 'workflow',
  values: PipelineNodeFormValues
): PipelineNodeDraftByType['workflow'];
export function buildPipelineNodeDraft(
  nodeType: 'prompt',
  values: PipelineNodeFormValues
): PipelineNodeDraftByType['prompt'];
export function buildPipelineNodeDraft(
  nodeType: 'sequentialThinking',
  values: PipelineNodeFormValues
): PipelineNodeDraftByType['sequentialThinking'];
export function buildPipelineNodeDraft(
  nodeType: 'tool',
  values: PipelineNodeFormValues
): PipelineNodeDraftByType['tool'];
export function buildPipelineNodeDraft(
  nodeType: PipelineNodeType,
  values: PipelineNodeFormValues
): PipelineNodeDraftByType[PipelineNodeType] {
  switch (nodeType) {
    case 'prompt':
      return {
        agent: trim(values.promptAgent),
        prompt: trim(values.promptTemplate),
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
      };
    case 'tool': {
      const parsedToolArgs = parsePipelineToolArgs(values.toolArgs);
      const normalizedToolArgs =
        parsedToolArgs && typeof parsedToolArgs === 'object'
          ? normalizeToolArgsRecord(parsedToolArgs)
          : parsedToolArgs;
      return {
        toolName: trim(values.toolName),
        ...(normalizedToolArgs ? { toolArgs: normalizedToolArgs } : {}),
      };
    }
    default:
      return {
        workflowId: trim(values.workflowId),
        agent: trim(values.workflowAgent),
        ...(trim(values.workflowPrompt) ? { prompt: trim(values.workflowPrompt) } : {}),
      };
  }
}
