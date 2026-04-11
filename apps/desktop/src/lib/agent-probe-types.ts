import type {
  AcpMetadata,
  AgentInfo,
  ModelInfo,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
  SessionMode,
} from './types';

export type AgentProbeStyle = 'modern' | 'unknown';
export type AgentProbeState = 'ready' | 'offline' | 'error';

export interface AgentProbePromptCapabilities {
  image: boolean;
  audio: boolean;
  embeddedContext: boolean;
}

export interface AgentProbeSessionCapabilities {
  list: boolean;
  loadSession: boolean;
}

export interface AgentProbeTransportCapabilities {
  promptCapabilities: AgentProbePromptCapabilities;
  sessionCapabilities: AgentProbeSessionCapabilities;
}

export type AgentProbeConfigValueType =
  | 'string'
  | 'boolean'
  | 'number'
  | 'json'
  | 'unknown';

export interface AgentProbeConfigOption {
  id: string;
  name: string;
  description?: string;
  category?: string;
  type?: string;
  currentValue?: string;
  rawCurrentValue?: unknown;
  valueType?: AgentProbeConfigValueType;
  options: SessionConfigSelectOptions;
  _meta?: AcpMetadata;
}

export interface AgentProbeResult {
  style: AgentProbeStyle;
  state: AgentProbeState;
  offline: boolean;
  modes: string[];
  models: string[];
  modeOptions: SessionMode[];
  modelOptions: ModelInfo[];
  configOptions: Record<string, AgentProbeConfigOption>;
  configOptionList: AgentProbeConfigOption[];
  modeConfigOption?: AgentProbeConfigOption;
  modelConfigOption?: AgentProbeConfigOption;
  thoughtLevelConfigOption?: AgentProbeConfigOption;
  agentInfo?: AgentInfo | null;
  agentCapabilities?: AgentProbeTransportCapabilities | null;
  probedAt: number;
  error?: string;
  refreshing?: boolean;
  degradedMessage?: string;
}

const BOOLEAN_CONFIG_OPTIONS: SessionConfigSelectOption[] = [
  { value: 'true', name: 'Enabled' },
  { value: 'false', name: 'Disabled' },
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeMetadata(value: unknown): AcpMetadata {
  return isRecord(value) ? value : null;
}

function normalizeString(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function normalizeBoolean(value: unknown): boolean {
  return value === true;
}

function normalizeSelectOption(value: unknown): SessionConfigSelectOption | null {
  if (typeof value === 'string' && value.length > 0) {
    return { value, name: value };
  }

  if (!isRecord(value)) {
    return null;
  }

  const optionValue = normalizeString(value.value);
  const optionName = normalizeString(value.name) ?? optionValue;

  if (!optionValue || !optionName) {
    return null;
  }

  return {
    value: optionValue,
    name: optionName,
    description: normalizeString(value.description),
    _meta: normalizeMetadata(value._meta),
  };
}

function normalizeSelectOptions(value: unknown): SessionConfigSelectOptions {
  if (!Array.isArray(value)) {
    return [];
  }

  const hasGroups = value.some(
    (entry) => isRecord(entry) && typeof entry.group === 'string' && Array.isArray(entry.options),
  );

  if (!hasGroups) {
    return value
      .map(normalizeSelectOption)
      .filter((entry): entry is SessionConfigSelectOption => entry !== null);
  }

  return value
    .filter((entry): entry is Record<string, unknown> => isRecord(entry))
    .map((entry): SessionConfigSelectGroup | null => {
      const group = normalizeString(entry.group);
      const name = normalizeString(entry.name) ?? group;

      if (!group || !name || !Array.isArray(entry.options)) {
        return null;
      }

      return {
        group,
        name,
        _meta: normalizeMetadata(entry._meta),
        options: entry.options
          .map(normalizeSelectOption)
          .filter((option): option is SessionConfigSelectOption => option !== null),
      };
    })
    .filter((entry): entry is SessionConfigSelectGroup => entry !== null);
}

function normalizeConfigValue(currentValue: unknown): Pick<
  AgentProbeConfigOption,
  'currentValue' | 'rawCurrentValue' | 'valueType'
> {
  if (typeof currentValue === 'string') {
    return { currentValue, rawCurrentValue: currentValue, valueType: 'string' };
  }

  if (typeof currentValue === 'boolean') {
    return {
      currentValue: String(currentValue),
      rawCurrentValue: currentValue,
      valueType: 'boolean',
    };
  }

  if (typeof currentValue === 'number' && Number.isFinite(currentValue)) {
    return {
      currentValue: String(currentValue),
      rawCurrentValue: currentValue,
      valueType: 'number',
    };
  }

  if (currentValue === null || currentValue === undefined) {
    return {};
  }

  return {
    currentValue: JSON.stringify(currentValue),
    rawCurrentValue: currentValue,
    valueType: 'json',
  };
}

function normalizeAgentInfo(value: unknown): AgentInfo | null {
  if (!isRecord(value)) {
    return null;
  }

  const name = normalizeString(value.name);
  if (!name) {
    return null;
  }

  return {
    name,
    title: normalizeString(value.title),
    version: normalizeString(value.version),
    _meta: normalizeMetadata(value._meta),
  };
}

function normalizeAgentCapabilities(value: unknown): AgentProbeTransportCapabilities | null {
  if (!isRecord(value)) {
    return null;
  }

  const promptCapabilities = isRecord(value.promptCapabilities)
    ? value.promptCapabilities
    : {};
  const sessionCapabilities = isRecord(value.sessionCapabilities)
    ? value.sessionCapabilities
    : {};

  return {
    promptCapabilities: {
      image: normalizeBoolean(promptCapabilities.image),
      audio: normalizeBoolean(promptCapabilities.audio),
      embeddedContext: normalizeBoolean(promptCapabilities.embeddedContext),
    },
    sessionCapabilities: {
      list: normalizeBoolean(sessionCapabilities.list),
      loadSession: normalizeBoolean(sessionCapabilities.loadSession),
    },
  };
}

function normalizeConfigOptionEntry(
  key: string | undefined,
  value: unknown,
): AgentProbeConfigOption | null {
  if (typeof value === 'string' && value.length > 0) {
    return {
      id: key ?? value,
      name: value,
      options: [],
    };
  }

  if (!isRecord(value)) {
    return null;
  }

  const id = normalizeString(value.id) ?? key;
  const name = normalizeString(value.name) ?? id;

  if (!id || !name) {
    return null;
  }

  const baseOptions = normalizeSelectOptions(value.options);
  const normalizedValue = normalizeConfigValue(value.currentValue);
  const declaredType = normalizeString(value.type);
  const valueType =
    normalizedValue.valueType
    ?? (declaredType === 'boolean' ? 'boolean' : undefined);

  return {
    id,
    name,
    description: normalizeString(value.description),
    category: normalizeString(value.category),
    type: declaredType,
    options:
      valueType === 'boolean' && baseOptions.length === 0
        ? BOOLEAN_CONFIG_OPTIONS.map((entry) => ({ ...entry }))
        : baseOptions,
    _meta: normalizeMetadata(value._meta),
    ...normalizedValue,
    ...(valueType ? { valueType } : {}),
  };
}

function normalizeConfigOptions(value: unknown): AgentProbeConfigOption[] {
  if (Array.isArray(value)) {
    return value
      .map((entry) => normalizeConfigOptionEntry(undefined, entry))
      .filter((entry): entry is AgentProbeConfigOption => entry !== null);
  }

  if (!isRecord(value)) {
    return [];
  }

  return Object.entries(value)
    .map(([key, entry]) => normalizeConfigOptionEntry(key, entry))
    .filter((entry): entry is AgentProbeConfigOption => entry !== null);
}

function flattenSelectOptions(options: SessionConfigSelectOptions): SessionConfigSelectOption[] {
  if (options.length === 0) {
    return [];
  }

  return 'group' in options[0]
    ? (options as SessionConfigSelectGroup[]).flatMap((group) => group.options)
    : (options as SessionConfigSelectOption[]);
}

function modeOptionsFromConfigOptions(
  options: AgentProbeConfigOption[],
): SessionMode[] {
  const modeOption = options.find((option) => option.category === 'mode');
  if (!modeOption) {
    return [];
  }

  return flattenSelectOptions(modeOption.options).map((option) => ({
    id: option.value,
    name: option.name,
    description: option.description,
    _meta: option._meta ?? null,
  }));
}

function modelOptionsFromConfigOptions(
  options: AgentProbeConfigOption[],
): ModelInfo[] {
  const modelOption = options.find((option) => option.category === 'model');
  if (!modelOption) {
    return [];
  }

  return flattenSelectOptions(modelOption.options).map((option) => ({
    modelId: option.value,
    name: option.name,
    description: option.description,
    _meta: option._meta ?? null,
  }));
}

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

function normalizeProbeStyle(
  value: unknown,
  hasConfigOptions: boolean,
): AgentProbeStyle {
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
