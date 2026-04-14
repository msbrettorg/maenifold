import { ref, type Ref } from 'vue';
import type {
  AgentProbeConfigOption,
  AgentProbeResult,
} from '../lib/agent-probe-types';
import type {
  PipelineNodeAgentConfig,
  PipelineNodeDraftByType,
  PipelineNodeType,
} from '../lib/pipeline-types';
import { normalizeNodeCwd } from '../lib/pipeline-node-factory';
import type {
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
} from '../lib/types';

export interface PipelineNodeFormValues {
  workflowId: string;
  workflowAgent: string;
  workflowAgentConfig: PipelineNodeAgentConfig;
  workflowPrompt: string;
  promptAgent: string;
  promptAgentConfig: PipelineNodeAgentConfig;
  promptTemplate: string;
  thinkingTopic: string;
  thinkingAgent: string;
  thinkingAgentConfig: PipelineNodeAgentConfig;
  thinkingTotalThoughts: string;
  thinkingAnalysisType: string;
  toolName: string;
  toolAgent: string;
  toolAgentConfig: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  toolArgsValid: boolean;
  cwd?: string;
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
  workflowAgentConfig: Ref<PipelineNodeAgentConfig>;
  workflowPrompt: Ref<string>;
  promptAgent: Ref<string>;
  promptAgentConfig: Ref<PipelineNodeAgentConfig>;
  promptTemplate: Ref<string>;
  thinkingTopic: Ref<string>;
  thinkingAgent: Ref<string>;
  thinkingAgentConfig: Ref<PipelineNodeAgentConfig>;
  thinkingTotalThoughts: Ref<string>;
  thinkingAnalysisType: Ref<string>;
  toolName: Ref<string>;
  toolAgent: Ref<string>;
  toolAgentConfig: Ref<PipelineNodeAgentConfig>;
  toolArgs: Ref<Record<string, unknown>>;
  toolArgsValid: Ref<boolean>;
  cwd: Ref<string>;
  getFormValues: () => PipelineNodeFormValues;
}

export function usePipelineNodeFormFields(): PipelineNodeFormRefs {
  const workflowId = ref('');
  const workflowAgent = ref('');
  const workflowAgentConfig = ref<PipelineNodeAgentConfig>({});
  const workflowPrompt = ref('');
  const promptAgent = ref('');
  const promptAgentConfig = ref<PipelineNodeAgentConfig>({});
  const promptTemplate = ref('');
  const thinkingTopic = ref('');
  const thinkingAgent = ref('');
  const thinkingAgentConfig = ref<PipelineNodeAgentConfig>({});
  const thinkingTotalThoughts = ref(DEFAULT_PIPELINE_NODE_THOUGHTS);
  const thinkingAnalysisType = ref('');
  const toolName = ref('');
  const toolAgent = ref('');
  const toolAgentConfig = ref<PipelineNodeAgentConfig>({});
  const toolArgs = ref<Record<string, unknown>>({});
  const toolArgsValid = ref(true);
  const cwd = ref('');

  return {
    workflowId,
    workflowAgent,
    workflowAgentConfig,
    workflowPrompt,
    promptAgent,
    promptAgentConfig,
    promptTemplate,
    thinkingTopic,
    thinkingAgent,
    thinkingAgentConfig,
    thinkingTotalThoughts,
    thinkingAnalysisType,
    toolName,
    toolAgent,
    toolAgentConfig,
    toolArgs,
    toolArgsValid,
    cwd,
    getFormValues: () => ({
      workflowId: workflowId.value,
      workflowAgent: workflowAgent.value,
      workflowAgentConfig: workflowAgentConfig.value,
      workflowPrompt: workflowPrompt.value,
      promptAgent: promptAgent.value,
      promptAgentConfig: promptAgentConfig.value,
      promptTemplate: promptTemplate.value,
      thinkingTopic: thinkingTopic.value,
      thinkingAgent: thinkingAgent.value,
      thinkingAgentConfig: thinkingAgentConfig.value,
      thinkingTotalThoughts: thinkingTotalThoughts.value,
      thinkingAnalysisType: thinkingAnalysisType.value,
      toolName: toolName.value,
      toolAgent: toolAgent.value,
      toolAgentConfig: toolAgentConfig.value,
      toolArgs: toolArgs.value,
      toolArgsValid: toolArgsValid.value,
      cwd: cwd.value,
    }),
  };
}

export function isSessionConfigSelectGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup,
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

export function flattenSessionConfigOptions(
  options: SessionConfigSelectOptions,
): SessionConfigSelectOption[] {
  return options.flatMap((entry) => (isSessionConfigSelectGroup(entry) ? entry.options : [entry]));
}

function stringifyAgentConfigValue(value: unknown): string {
  if (typeof value === 'string') {
    return value;
  }

  if (typeof value === 'boolean' || (typeof value === 'number' && Number.isFinite(value))) {
    return String(value);
  }

  if (value == null) {
    return '';
  }

  try {
    return JSON.stringify(value);
  } catch {
    return '';
  }
}

function cloneConfigOptions(
  value: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  return value ? { ...value } : undefined;
}

export function normalizePipelineNodeAgentConfig(
  value: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig | undefined {
  if (!value) {
    return undefined;
  }

  const model = typeof value.model === 'string' ? trim(value.model) : '';
  const mode = typeof value.mode === 'string' ? trim(value.mode) : '';
  const thoughtLevel = typeof value.thoughtLevel === 'string' ? trim(value.thoughtLevel) : '';

  const normalized: PipelineNodeAgentConfig = {
    ...(model ? { model } : {}),
    ...(mode ? { mode } : {}),
    ...(thoughtLevel ? { thoughtLevel } : {}),
    ...(value.configOptions && Object.keys(value.configOptions).length > 0
      ? { configOptions: cloneConfigOptions(value.configOptions) }
      : {}),
  };

  return Object.keys(normalized).length > 0 ? normalized : undefined;
}

export function clonePipelineNodeAgentConfig(
  value: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig {
  return normalizePipelineNodeAgentConfig(value) ?? {};
}

function isThoughtLevelConfigOption(option: AgentProbeConfigOption): boolean {
  return option.category === 'thought_level'
    || option.id === 'thought_level'
    || option.id === 'reasoning_effort';
}

export function buildAgentConfigDefaults(
  probe: AgentProbeResult | undefined,
): PipelineNodeAgentConfig | undefined {
  if (!probe || probe.state !== 'ready') {
    return undefined;
  }

  const configOptions = Object.fromEntries(
    probe.configOptionList
      .filter((option) =>
        option.id !== probe.modeConfigOption?.id
        && option.id !== probe.modelConfigOption?.id
        && option.id !== probe.thoughtLevelConfigOption?.id
        && option.currentValue !== undefined,
      )
      .map((option) => [option.id, option.rawCurrentValue ?? option.currentValue]),
  );

  return normalizePipelineNodeAgentConfig({
    ...(probe.modeConfigOption?.currentValue ? { mode: probe.modeConfigOption.currentValue } : {}),
    ...(probe.modelConfigOption?.currentValue ? { model: probe.modelConfigOption.currentValue } : {}),
    ...(probe.thoughtLevelConfigOption?.currentValue
      ? { thoughtLevel: probe.thoughtLevelConfigOption.currentValue }
      : {}),
    ...(Object.keys(configOptions).length > 0 ? { configOptions } : {}),
  });
}

export function formatPipelineAgentOptionLabel(
  agentName: string,
  probe: AgentProbeResult | undefined,
): string {
  if (!probe) return agentName;
  if (probe.error) return `${agentName} ⚠`;
  if (probe.modes.length <= 2 && probe.modes.length > 0) return `${agentName} — ${probe.modes.join(', ')}`;
  if (probe.modes.length > 2) return `${agentName} (${probe.modes.length} modes)`;
  return probe.models.length
    ? `${agentName} (${probe.models.length} model${probe.models.length === 1 ? '' : 's'})`
    : agentName;
}

export function getPipelineAgentCapabilityBadges(
  probe: AgentProbeResult | undefined,
): string[] {
  if (!probe) return [];
  if (probe.error) return ['⚠ probe failed'];
  return [
    ...probe.modes,
    ...(probe.models.length
      ? [probe.models.length === 1 ? probe.models[0] : `${probe.models.length} models`]
      : []),
  ];
}

export function getEditableAgentConfigOptions(
  probe: AgentProbeResult | undefined,
): AgentProbeConfigOption[] {
  if (!probe || probe.state !== 'ready') {
    return [];
  }

  return probe.configOptionList.filter(
    (option) =>
      option.id !== probe.modeConfigOption?.id
      && option.id !== probe.modelConfigOption?.id
      && option.id !== probe.thoughtLevelConfigOption?.id,
  );
}

export function getAgentModeValue(
  config: PipelineNodeAgentConfig | undefined,
  probe: AgentProbeResult | undefined,
): string {
  return config?.mode ?? probe?.modeConfigOption?.currentValue ?? probe?.modeOptions[0]?.id ?? '';
}

export function getAgentModelValue(
  config: PipelineNodeAgentConfig | undefined,
  probe: AgentProbeResult | undefined,
): string {
  return config?.model
    ?? probe?.modelConfigOption?.currentValue
    ?? probe?.modelOptions[0]?.modelId
    ?? '';
}

export function getAgentThoughtLevelValue(
  config: PipelineNodeAgentConfig | undefined,
  probe: AgentProbeResult | undefined,
): string {
  return config?.thoughtLevel ?? probe?.thoughtLevelConfigOption?.currentValue ?? '';
}

export function getAgentConfigOptionValue(
  config: PipelineNodeAgentConfig | undefined,
  option: AgentProbeConfigOption,
): string {
  return stringifyAgentConfigValue(config?.configOptions?.[option.id] ?? option.rawCurrentValue ?? option.currentValue);
}

export function setAgentModeValue(
  config: PipelineNodeAgentConfig | undefined,
  value: string,
): PipelineNodeAgentConfig | undefined {
  const next = clonePipelineNodeAgentConfig(config);
  if (trim(value)) next.mode = trim(value);
  else delete next.mode;
  return normalizePipelineNodeAgentConfig(next);
}

export function setAgentModelValue(
  config: PipelineNodeAgentConfig | undefined,
  value: string,
): PipelineNodeAgentConfig | undefined {
  const next = clonePipelineNodeAgentConfig(config);
  if (trim(value)) next.model = trim(value);
  else delete next.model;
  return normalizePipelineNodeAgentConfig(next);
}

export function setAgentThoughtLevelValue(
  config: PipelineNodeAgentConfig | undefined,
  value: string,
): PipelineNodeAgentConfig | undefined {
  const next = clonePipelineNodeAgentConfig(config);
  if (trim(value)) next.thoughtLevel = trim(value);
  else delete next.thoughtLevel;
  return normalizePipelineNodeAgentConfig(next);
}

function coerceAgentConfigOptionValue(
  option: AgentProbeConfigOption,
  value: string,
): unknown {
  if (option.valueType === 'boolean' || option.type === 'boolean') {
    return value === 'true';
  }

  if (option.valueType === 'number') {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : value;
  }

  if (option.valueType === 'json') {
    try {
      return JSON.parse(value);
    } catch {
      return value;
    }
  }

  return value;
}

export function setAgentConfigOptionValue(
  config: PipelineNodeAgentConfig | undefined,
  option: AgentProbeConfigOption,
  value: string,
): PipelineNodeAgentConfig | undefined {
  if (option.category === 'mode') {
    return setAgentModeValue(config, value);
  }

  if (option.category === 'model') {
    return setAgentModelValue(config, value);
  }

  if (isThoughtLevelConfigOption(option)) {
    return setAgentThoughtLevelValue(config, value);
  }

  const next = clonePipelineNodeAgentConfig(config);
  const configOptions = { ...(next.configOptions ?? {}) };

  if (!trim(value)) {
    delete configOptions[option.id];
  } else {
    configOptions[option.id] = coerceAgentConfigOptionValue(option, value);
  }

  if (Object.keys(configOptions).length > 0) {
    next.configOptions = configOptions;
  } else {
    delete next.configOptions;
  }

  return normalizePipelineNodeAgentConfig(next);
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
