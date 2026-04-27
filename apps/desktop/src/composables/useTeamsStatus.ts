// Backend contract lock — every type/field in this file MUST mirror one of:
//   * crates/maenifold-core/src/acp/config/teams_monitor.rs
//       (TeamsMonitorConfig, TeamsMonitorIdentityConfig, TeamsMonitorConnectionConfig)
//   * crates/maenifold-core/src/tools/teams_monitor_helpers.rs::status_value
//       (status payload returned by every TeamsMonitor action)
//   * crates/maenifold-core/src/mcp/resources/teams.rs (teams://status, teams://config)
// Do NOT add fields without a backend counterpart. The Settings UI binds
// directly to these shapes; phantom fields ship as silent no-ops.
import type { ResourceContent } from './mcpTransport';

export const TEAMS_STATUS_URI = 'teams://status';

export type TeamsConnectionState =
  | 'disabled'
  | 'connecting'
  | 'connected'
  | 'authenticated'
  | 'pending_auth'
  | 'reconnecting'
  | 'unauthenticated'
  | 'error';

export type TeamsAuthState = 'unauthenticated' | 'pending' | 'authenticated' | 'expired' | 'error';

export interface TeamsStatus {
  enabled: boolean;
  connectionState: TeamsConnectionState;
  lastMessageAt: string | null;
  lastMatchAt: string | null;
  matchCount: number;
  activeDispatches: number;
  lastError: string | null;
  authState: TeamsAuthState;
  expiresAt: string | null;
  lastAuthError: string | null;
}

export interface TeamsIdentity {
  mri: string;
  objectId: string;
  tenantId?: string | null;
  upn?: string | null;
  displayName?: string | null;
  expiresAt: string;
  chatsvcRegion?: string | null;
  source: string;
}

export interface TeamsConnectionConfig {
  chatsvcRegion?: string;
  trouterGateway?: string;
  registrarUrl?: string;
  appId?: string;
  reconnectDelaySeconds?: number;
  maxReconnectDelaySeconds?: number;
  tokenRefreshMinutes?: number;
  heartbeatIntervalSeconds?: number;
}

export interface TeamsMonitorConfig {
  identity?: TeamsIdentity | null;
  keyword?: string;
  listenScope?: string;
  monitoredConversations?: string[];
  replyBehavior?: string;
  replyPrefix?: string;
  dispatchTimeoutMinutes?: number;
  dispatchAgent?: string | null;
  connection?: TeamsConnectionConfig;
}

export interface TeamsMonitorStatus extends Partial<TeamsStatus> {
  config?: TeamsMonitorConfig;
  message?: string;
}

const DEFAULT_TEAMS_STATUS: TeamsStatus = {
  enabled: false,
  connectionState: 'disabled',
  lastMessageAt: null,
  lastMatchAt: null,
  matchCount: 0,
  activeDispatches: 0,
  lastError: null,
  authState: 'unauthenticated',
  expiresAt: null,
  lastAuthError: null,
};

const VALID_CONNECTION_STATES = new Set<TeamsConnectionState>([
  'disabled',
  'connecting',
  'connected',
  'authenticated',
  'pending_auth',
  'reconnecting',
  'unauthenticated',
  'error',
]);

const VALID_AUTH_STATES = new Set<TeamsAuthState>([
  'unauthenticated',
  'pending',
  'authenticated',
  'expired',
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
    authState: parseAuthState(obj.authState),
    expiresAt: stringOrNull(obj.expiresAt),
    lastAuthError: stringOrNull(obj.lastAuthError),
  };
}

export function formatTeamsConnectionState(value: string): string {
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

function parseAuthState(value: unknown): TeamsAuthState {
  if (typeof value === 'string' && VALID_AUTH_STATES.has(value as TeamsAuthState)) {
    return value as TeamsAuthState;
  }
  return 'unauthenticated';
}

function stringOrNull(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function nonNegativeInteger(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? Math.floor(value)
    : 0;
}
