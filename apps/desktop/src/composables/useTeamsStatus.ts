import type { ResourceContent } from './mcpTransport';

export const TEAMS_STATUS_URI = 'teams://status';

export type TeamsConnectionState = 'disabled' | 'connecting' | 'connected' | 'reconnecting' | 'error';

export interface TeamsStatus {
  enabled: boolean;
  connectionState: TeamsConnectionState;
  lastMessageAt: string | null;
  lastMatchAt: string | null;
  matchCount: number;
  activeDispatches: number;
  lastError: string | null;
}

const DEFAULT_TEAMS_STATUS: TeamsStatus = {
  enabled: false,
  connectionState: 'disabled',
  lastMessageAt: null,
  lastMatchAt: null,
  matchCount: 0,
  activeDispatches: 0,
  lastError: null,
};

const VALID_CONNECTION_STATES = new Set<TeamsConnectionState>([
  'disabled',
  'connecting',
  'connected',
  'reconnecting',
  'error',
]);

export function defaultTeamsStatus(overrides: Partial<TeamsStatus> = {}): TeamsStatus {
  return { ...DEFAULT_TEAMS_STATUS, ...overrides };
}

export function parseTeamsStatusResource(content: ResourceContent): TeamsStatus {
  if (typeof content.text !== 'string' || content.text.length === 0) {
    throw new Error(`Teams status resource ${content.uri} returned no JSON text.`);
  }
  return parseTeamsStatusPayload(JSON.parse(content.text));
}

export function parseTeamsStatusPayload(payload: unknown): TeamsStatus {
  if (!payload || typeof payload !== 'object') {
    return defaultTeamsStatus();
  }

  const obj = payload as Record<string, unknown>;
  const connectionState = parseConnectionState(obj.connectionState);
  const enabled = typeof obj.enabled === 'boolean'
    ? obj.enabled
    : connectionState !== 'disabled';

  return {
    enabled,
    connectionState,
    lastMessageAt: stringOrNull(obj.lastMessageAt),
    lastMatchAt: stringOrNull(obj.lastMatchAt),
    matchCount: nonNegativeInteger(obj.matchCount),
    activeDispatches: nonNegativeInteger(obj.activeDispatches),
    lastError: stringOrNull(obj.lastError),
  };
}

export function formatTeamsConnectionState(value: TeamsConnectionState): string {
  return value
    .split('_')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function parseConnectionState(value: unknown): TeamsConnectionState {
  if (typeof value === 'string' && VALID_CONNECTION_STATES.has(value as TeamsConnectionState)) {
    return value as TeamsConnectionState;
  }
  return 'disabled';
}

function stringOrNull(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function nonNegativeInteger(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? Math.floor(value)
    : 0;
}
