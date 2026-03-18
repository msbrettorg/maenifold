// Pure helper functions shared across session composables
import type {
  SessionConfigOption,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
  SessionState,
} from '../lib/types';

// Startup phase detection patterns
export function detectPhase(line: string): string | null {
  const lower = line.toLowerCase();
  if (lower.includes('download') || lower.includes('fetch') || lower.includes('get ')) {
    return 'downloading';
  }
  if (lower.includes('install') || lower.includes('added') || lower.includes('packages')) {
    return 'installing';
  }
  if (lower.includes('build') || lower.includes('compil')) {
    return 'building';
  }
  if (lower.includes('start') || lower.includes('spawn')) {
    return 'starting';
  }
  return null;
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

export function normalizeConfigSelectOption(option: unknown): SessionConfigSelectOption | null {
  if (!isRecord(option) || typeof option.value !== 'string' || typeof option.name !== 'string') {
    return null;
  }

  return {
    value: option.value,
    name: option.name,
    description: typeof option.description === 'string' ? option.description : undefined,
  };
}

export function normalizeConfigSelectOptions(options: unknown): SessionConfigSelectOptions {
  if (!Array.isArray(options)) {
    return [];
  }

  const hasGroups = options.some(
    (option) => isRecord(option) && typeof option.group === 'string' && Array.isArray(option.options)
  );

  if (hasGroups) {
    return options
      .filter((option): option is Record<string, unknown> => isRecord(option))
      .map((group): SessionConfigSelectGroup | null => {
        if (
          typeof group.group !== 'string' ||
          typeof group.name !== 'string' ||
          !Array.isArray(group.options)
        ) {
          return null;
        }

        return {
          group: group.group,
          name: group.name,
          options: group.options
            .map(normalizeConfigSelectOption)
            .filter((option): option is SessionConfigSelectOption => option !== null),
        };
      })
      .filter((group): group is SessionConfigSelectGroup => group !== null);
  }

  return options
    .map(normalizeConfigSelectOption)
    .filter((option): option is SessionConfigSelectOption => option !== null);
}

export function normalizeConfigOptions(configOptions: unknown): SessionConfigOption[] {
  if (!Array.isArray(configOptions)) {
    return [];
  }

  return configOptions
    .filter(
      (option): option is Record<string, unknown> =>
        isRecord(option) &&
        option.type === 'select' &&
        typeof option.id === 'string' &&
        typeof option.name === 'string' &&
        typeof option.currentValue === 'string'
    )
    .map((option) => ({
      id: option.id as string,
      name: option.name as string,
      description: typeof option.description === 'string' ? option.description : undefined,
      category: typeof option.category === 'string' ? option.category : undefined,
      type: 'select' as const,
      currentValue: option.currentValue as string,
      options: normalizeConfigSelectOptions(option.options),
    }));
}

export function parseUpdatedAt(updatedAt?: string | null): number | undefined {
  if (!updatedAt) {
    return undefined;
  }

  const timestamp = new Date(updatedAt).getTime();
  return Number.isNaN(timestamp) ? undefined : timestamp;
}

export function applyConfigOptionsState(
  session: SessionState,
  response: { configOptions?: unknown[] | null }
): void {
  session.hasConfigOptions =
    'configOptions' in response && response.configOptions !== null;
  session.configOptions = session.hasConfigOptions
    ? normalizeConfigOptions(response.configOptions)
    : [];
}

export function applyLegacySessionState(
  session: SessionState,
  response: {
    modes?: {
      availableModes?: Array<{ id: string; name: string; description?: string | null }>;
      currentModeId?: string | null;
    } | null;
    models?: {
      availableModels?: Array<{ modelId: string; name: string; description?: string | null }>;
      currentModelId?: string | null;
    } | null;
  }
): void {
  if (session.hasConfigOptions) {
    session.availableModes = [];
    session.currentModeId = '';
    session.availableModels = [];
    session.currentModelId = '';
    return;
  }

  if (response.modes) {
    session.availableModes = (response.modes.availableModes || []).map(m => ({
      id: m.id,
      name: m.name,
      description: m.description ?? undefined,
    }));
    session.currentModeId = response.modes.currentModeId || '';
  } else {
    session.availableModes = [];
    session.currentModeId = '';
  }

  if (response.models) {
    session.availableModels = (response.models.availableModels || []).map(m => ({
      modelId: m.modelId,
      name: m.name,
      description: m.description ?? undefined,
    }));
    session.currentModelId = response.models.currentModelId || '';
  } else {
    session.availableModels = [];
    session.currentModelId = '';
  }
}

export function applySessionConfigurationState(
  session: SessionState,
  response: {
    configOptions?: unknown[] | null;
    modes?: {
      availableModes?: Array<{ id: string; name: string; description?: string | null }>;
      currentModeId?: string | null;
    } | null;
    models?: {
      availableModels?: Array<{ modelId: string; name: string; description?: string | null }>;
      currentModelId?: string | null;
    } | null;
  }
): void {
  applyConfigOptionsState(session, response);
  applyLegacySessionState(session, response);
}
