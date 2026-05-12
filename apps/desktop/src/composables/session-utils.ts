import type {
  AvailableCommand,
  SavedSession,
  SessionConfigOption,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
  SessionMode,
  SessionState,
} from '../lib/types';

const BOOLEAN_CONFIG_OPTIONS: SessionConfigSelectOption[] = [
  { value: 'true', name: 'Enabled' },
  { value: 'false', name: 'Disabled' },
];
const LEGACY_TIMESTAMP_SESSION_PREFIX = 'New session -';
const UNTITLED_SESSION_NAME = 'Untitled session';

export function detectPhase(line: string): string | null {
  const lower = line.toLowerCase();
  if (lower.includes('download') || lower.includes('fetch') || lower.includes('get ')) return 'downloading';
  if (lower.includes('install') || lower.includes('added') || lower.includes('packages')) return 'installing';
  if (lower.includes('build') || lower.includes('compil')) return 'building';
  if (lower.includes('start') || lower.includes('spawn')) return 'starting';
  return null;
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function normalizeConfigSelectOption(option: unknown): SessionConfigSelectOption | null {
  if (!isRecord(option) || typeof option.value !== 'string' || typeof option.name !== 'string') return null;
  return {
    value: option.value,
    name: option.name,
    description: typeof option.description === 'string' ? option.description : undefined,
    _meta: isRecord(option._meta) ? option._meta : null,
  };
}

function normalizeConfigSelectOptions(options: unknown): SessionConfigSelectOptions {
  if (!Array.isArray(options)) return [];
  const hasGroups = options.some(
    option => isRecord(option) && typeof option.group === 'string' && Array.isArray(option.options)
  );
  if (!hasGroups) {
    return options
      .map(normalizeConfigSelectOption)
      .filter((option): option is SessionConfigSelectOption => option !== null);
  }
  return options
    .filter((option): option is Record<string, unknown> => isRecord(option))
    .map((group): SessionConfigSelectGroup | null => {
      if (typeof group.group !== 'string' || typeof group.name !== 'string' || !Array.isArray(group.options)) {
        return null;
      }
      return {
        group: group.group,
        name: group.name,
        _meta: isRecord(group._meta) ? group._meta : null,
        options: group.options
          .map(normalizeConfigSelectOption)
          .filter((option): option is SessionConfigSelectOption => option !== null),
      };
    })
    .filter((group): group is SessionConfigSelectGroup => group !== null);
}

function normalizeConfigValue(currentValue: unknown): Pick<
  SessionConfigOption,
  'currentValue' | 'rawCurrentValue' | 'valueType'
> | null {
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
  return null;
}

export function normalizeConfigOptions(configOptions: unknown): SessionConfigOption[] {
  if (!Array.isArray(configOptions)) return [];
  return configOptions.flatMap((option) => {
    if (!isRecord(option) || typeof option.id !== 'string' || typeof option.name !== 'string') return [];
    const value = normalizeConfigValue(option.currentValue);
    if (!value) return [];
    const isBooleanOption = option.type === 'boolean' || value.valueType === 'boolean';
    return [{
      id: option.id,
      name: option.name,
      description: typeof option.description === 'string' ? option.description : undefined,
      category: typeof option.category === 'string' ? option.category : undefined,
      type: 'select' as const,
      options: isBooleanOption ? BOOLEAN_CONFIG_OPTIONS.map(entry => ({ ...entry })) : normalizeConfigSelectOptions(option.options),
      _meta: isRecord(option._meta) ? option._meta : null,
      ...value,
    }];
  });
}

export function parseUpdatedAt(updatedAt?: string | null): number | undefined {
  if (!updatedAt) return undefined;
  const timestamp = new Date(updatedAt).getTime();
  return Number.isNaN(timestamp) ? undefined : timestamp;
}

function normalizeAgentSessionName(agentName: string): string {
  return agentName.trim() || 'Agent';
}

export function createDefaultSessionTitle(agentName: string, savedSessions: SavedSession[]): string {
  const normalizedAgentName = normalizeAgentSessionName(agentName);
  const nextIndex =
    savedSessions.filter(
      (session) => normalizeAgentSessionName(session.agentName) === normalizedAgentName,
    ).length + 1;
  return `${normalizedAgentName} Session #${nextIndex}`;
}

export function deriveSessionTitleFromFirstMessage(text: string, maxLength = 40): string | null {
  const normalized = text.trim();
  if (!normalized) {
    return null;
  }

  return normalized.slice(0, maxLength);
}

export function isGeneratedSessionTitle(title: string, agentName: string): boolean {
  const normalizedTitle = title.trim();
  if (!normalizedTitle || normalizedTitle === UNTITLED_SESSION_NAME) {
    return true;
  }

  const normalizedAgentName = normalizeAgentSessionName(agentName);
  return (
    normalizedTitle.startsWith(`${normalizedAgentName} Session #`) ||
    normalizedTitle.startsWith(LEGACY_TIMESTAMP_SESSION_PREFIX) ||
    /^Session \d/.test(normalizedTitle)
  );
}

function applyConfigOptionsState(
  session: SessionState,
  response: { configOptions?: unknown[] | null }
): void {
  session.hasConfigOptions = 'configOptions' in response && response.configOptions !== null;
  session.configOptions = session.hasConfigOptions ? normalizeConfigOptions(response.configOptions) : [];
}

function normalizeSessionMode(mode: unknown): SessionMode | null {
  if (!isRecord(mode) || typeof mode.id !== 'string' || typeof mode.name !== 'string') return null;
  return {
    id: mode.id,
    name: mode.name,
    description: typeof mode.description === 'string' ? mode.description : undefined,
    _meta: isRecord(mode._meta) ? mode._meta : null,
  };
}

function applyModeState(
  session: SessionState,
  response: { modes?: unknown | null }
): void {
  if (!('modes' in response)) return;
  const modeSession = session as SessionState & { modes?: SessionMode[] };
  const modeState = response.modes;
  if (!isRecord(modeState)) {
    modeSession.modes = [];
    return;
  }
  modeSession.modes = Array.isArray(modeState.availableModes)
    ? modeState.availableModes.map(normalizeSessionMode).filter((mode): mode is SessionMode => mode !== null)
    : [];
  if (typeof modeState.currentModeId === 'string') {
    session.currentModeId = modeState.currentModeId;
  }
}

export function applySessionConfigurationState(
  session: SessionState,
  response: {
    configOptions?: unknown[] | null;
    availableCommands?: unknown[] | null;
    modes?: unknown | null;
  }
): void {
  applyConfigOptionsState(session, response);
  applyAvailableCommandsState(session, response);
  applyModeState(session, response);
}

function applyAvailableCommandsState(
  session: SessionState,
  response: { availableCommands?: unknown[] | null }
): void {
  if (Array.isArray(response.availableCommands)) {
    session.availableCommands = response.availableCommands as AvailableCommand[];
  }
}
