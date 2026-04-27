// Agent-config helpers for pipeline node form. Extracted from
// pipeline-node-form-helpers.ts to keep modules under 300 LOC.
import type {
  AgentProbeConfigOption,
  AgentProbeResult,
} from '../lib/agent-probe-types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';

const trim = (value: string) => value.trim();

function stringifyAgentConfigValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'boolean' || (typeof value === 'number' && Number.isFinite(value))) {
    return String(value);
  }
  if (value == null) return '';
  try { return JSON.stringify(value); }
  catch { return ''; }
}

function cloneConfigOptions(
  value: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  return value ? { ...value } : undefined;
}

export function normalizePipelineNodeAgentConfig(
  value: PipelineNodeAgentConfig | undefined,
): PipelineNodeAgentConfig | undefined {
  if (!value) return undefined;

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
  if (!probe || probe.state !== 'ready') return undefined;

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
  if (probe.error) return ['⚠ needs attention'];
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
  if (!probe || probe.state !== 'ready') return [];
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
  return stringifyAgentConfigValue(
    config?.configOptions?.[option.id] ?? option.rawCurrentValue ?? option.currentValue,
  );
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
  if (option.valueType === 'boolean' || option.type === 'boolean') return value === 'true';
  if (option.valueType === 'number') {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : value;
  }
  if (option.valueType === 'json') {
    try { return JSON.parse(value); } catch { return value; }
  }
  return value;
}

export function setAgentConfigOptionValue(
  config: PipelineNodeAgentConfig | undefined,
  option: AgentProbeConfigOption,
  value: string,
): PipelineNodeAgentConfig | undefined {
  if (option.category === 'mode') return setAgentModeValue(config, value);
  if (option.category === 'model') return setAgentModelValue(config, value);
  if (isThoughtLevelConfigOption(option)) return setAgentThoughtLevelValue(config, value);

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
