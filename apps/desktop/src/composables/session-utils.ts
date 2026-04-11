import type {
  AvailableCommand,
  SessionConfigOption,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
  SessionState,
} from '../lib/types';

const BOOLEAN_CONFIG_OPTIONS: SessionConfigSelectOption[] = [
  { value: 'true', name: 'Enabled' },
  { value: 'false', name: 'Disabled' },
];

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

function applyConfigOptionsState(
  session: SessionState,
  response: { configOptions?: unknown[] | null }
): void {
  session.hasConfigOptions = 'configOptions' in response && response.configOptions !== null;
  session.configOptions = session.hasConfigOptions ? normalizeConfigOptions(response.configOptions) : [];
  session.availableModes = [];
  session.currentModeId = '';
  session.availableModels = [];
  session.currentModelId = '';
}

export function applySessionConfigurationState(
  session: SessionState,
  response: {
    configOptions?: unknown[] | null;
    availableCommands?: unknown[] | null;
  }
): void {
  applyConfigOptionsState(session, response);
  applyAvailableCommandsState(session, response);
}

function applyAvailableCommandsState(
  session: SessionState,
  response: { availableCommands?: unknown[] | null }
): void {
  if (Array.isArray(response.availableCommands) && response.availableCommands.length > 0) {
    session.availableCommands = response.availableCommands as AvailableCommand[];
    return;
  }
  if (session.availableCommands.length > 0) return;
  synthesizeClientCommands(session);
}

function synthesizeClientCommands(session: SessionState): void {
  const commands: AvailableCommand[] = [];
  if (session.hasConfigOptions) {
    const modeOption = session.configOptions.find(
      opt => opt.category === 'mode' && opt.valueType !== 'boolean'
    );
    if (modeOption) {
      for (const opt of flatConfigSelectOptions(modeOption.options)) {
        commands.push({
          name: deriveCommandName(opt.value),
          description: `Switch to ${opt.name} mode`,
          _meta: { clientAction: 'setConfigOption', configId: modeOption.id, value: opt.value },
        });
      }
    }
  }
  commands.push({ name: 'clear', description: 'Clear chat messages', _meta: { clientAction: 'clearMessages' } });
  session.availableCommands = commands;
}

function deriveCommandName(value: string): string {
  const fragment = value.split('#').pop() || '';
  return fragment || value.split('/').pop() || value;
}

function flatConfigSelectOptions(options: SessionConfigSelectOptions): SessionConfigSelectOption[] {
  if (options.length === 0) return [];
  return 'group' in options[0]
    ? (options as SessionConfigSelectGroup[]).flatMap(group => group.options)
    : (options as SessionConfigSelectOption[]);
}

export function isClientCommand(command: AvailableCommand): boolean {
  return Boolean(command._meta && typeof command._meta === 'object' && 'clientAction' in command._meta);
}
