import type {
  AgentProbeResult,
  AgentProbeState,
  AgentProbeStyle,
} from './agent-probe-types-defs';
import {
  isRecord,
  modeOptionsFromConfigOptions,
  modelOptionsFromConfigOptions,
  normalizeAgentCapabilities,
  normalizeAgentInfo,
  normalizeConfigOptions,
  normalizeString,
} from './agent-probe-normalizers-base';

const OFFLINE_PROBE_ERROR_PATTERNS = [
  'not connected',
  'network error',
  'fetch',
  'timed out',
  'http 404',
  'channel closed',
  'connection refused',
  'broken pipe',
  'pipe closed',
  'socket hang up',
  'transport closed',
  'econnrefused',
  'econnreset',
  'enoent',
  'spawn failed',
  'failed to spawn',
  'failed to initialize',
  'initialization failed',
  'init failed',
  'handshake failed',
];

export function isOfflineProbeError(error?: string): boolean {
  if (!error) {
    return false;
  }

  const normalized = error.toLowerCase();
  return OFFLINE_PROBE_ERROR_PATTERNS.some((pattern) => normalized.includes(pattern));
}

function classifyProbeState(error?: string): AgentProbeState {
  if (!error) {
    return 'ready';
  }

  return isOfflineProbeError(error) ? 'offline' : 'error';
}

function normalizeProbeStyle(value: unknown, hasConfigOptions: boolean): AgentProbeStyle {
  if (value === 'modern') {
    return 'modern';
  }

  return hasConfigOptions ? 'modern' : 'unknown';
}

export function normalizeAgentProbeResult(
  value: unknown,
  options: { probedAt?: number; error?: string } = {},
): AgentProbeResult {
  const source = isRecord(value) ? value : {};
  const embeddedError = normalizeString(source.error);
  const error = options.error ?? embeddedError;
  const state = classifyProbeState(error);
  const configOptionList =
    state === 'ready' ? normalizeConfigOptions(source.configOptions) : [];
  const modeOptions = modeOptionsFromConfigOptions(configOptionList);
  const modelOptions = modelOptionsFromConfigOptions(configOptionList);
  const configOptions = Object.fromEntries(
    configOptionList.map((option) => [option.id, option]),
  );

  return {
    style: normalizeProbeStyle(source.style, configOptionList.length > 0),
    state,
    offline: state === 'offline',
    modes: modeOptions.map((option) => option.id),
    models: modelOptions.map((option) => option.modelId),
    modeOptions,
    modelOptions,
    configOptions,
    configOptionList,
    modeConfigOption: configOptionList.find((option) => option.category === 'mode'),
    modelConfigOption: configOptionList.find((option) => option.category === 'model'),
    thoughtLevelConfigOption: configOptionList.find(
      (option) =>
        option.category === 'thought_level'
        || option.id === 'thought_level'
        || option.id === 'reasoning_effort',
    ),
    agentInfo: normalizeAgentInfo(source.agentInfo),
    agentCapabilities: normalizeAgentCapabilities(source.agentCapabilities),
    probedAt: options.probedAt ?? Date.now(),
    ...(error ? { error } : {}),
  };
}
