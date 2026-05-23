// Pure helpers for agent capability probing. Extracted from agent-capabilities.ts.

import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import {
  isOfflineProbeError,
  normalizeAgentProbeResult,
} from '../lib/agent-probe-normalizers';
import type { AgentProbeResult } from '../lib/agent-probe-types-defs';
import type { McpAppUiMetadata } from '../lib/types';
import type { usePipelineMcp } from '../composables/usePipelineMcp';

export const AGENT_PROBE_CACHE_TTL_MS = 5 * 60_000;
export const AGENT_PROBE_SOFT_TIMEOUT_MS = 60_000;
export const AGENT_PROBE_TIMEOUT_MESSAGE =
  `GetConfig probe timed out after ${AGENT_PROBE_SOFT_TIMEOUT_MS / 1_000}s`;
export const AGENT_PROBE_FALLBACK_WARNING =
  'Live agent capability probe timed out. Showing configured agents and any cached capabilities until refresh succeeds.';

export interface ProbeAgentRecord {
  name?: string;
  agent?: string;
  id?: string;
  capabilities?: {
    error?: unknown;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeAppUiRecord(value: unknown): McpAppUiMetadata | null {
  if (!isRecord(value)) {
    return null;
  }

  const resourceUri = value.resourceUri;
  if (typeof resourceUri !== 'string' || !resourceUri.startsWith('ui://')) {
    return null;
  }

  return {
    resourceUri,
    originatingServer: typeof value.originatingServer === 'string' ? value.originatingServer : undefined,
    csp: typeof value.csp === 'string' ? value.csp : undefined,
    permissions: Array.isArray(value.permissions)
      ? value.permissions.filter((permission): permission is string => typeof permission === 'string')
      : undefined,
  };
}

export function extractAppUiMetadata(raw: Record<string, unknown>): McpAppUiMetadata | null {
  const directAppUi = normalizeAppUiRecord(raw.appUi);
  if (directAppUi) {
    return directAppUi;
  }

  const meta = raw._meta;
  if (!isRecord(meta)) return null;
  const ui = meta.ui;
  return normalizeAppUiRecord(ui);
}

export function toProbeResult(record?: ProbeAgentRecord, error?: string): AgentProbeResult {
  const source = isRecord(record?.capabilities) ? record.capabilities : record;
  return normalizeAgentProbeResult(source, { error, probedAt: Date.now() });
}

export function extractAgentRecords(result: unknown): ProbeAgentRecord[] {
  const data = unwrapToolData<unknown>(result);

  if (Array.isArray(data)) {
    return data.filter(isRecord) as ProbeAgentRecord[];
  }

  if (!isRecord(data)) {
    return [];
  }

  const nestedArrays = ['agents', 'data', 'results', 'items']
    .map((key) => data[key])
    .find(Array.isArray);

  if (Array.isArray(nestedArrays)) {
    return nestedArrays.filter(isRecord) as ProbeAgentRecord[];
  }

  if (isRecord(data.agents)) {
    return Object.entries(data.agents).map(([name, value]) => ({
      name,
      ...(isRecord(value) ? value : {}),
    }));
  }

  if ('name' in data || 'agent' in data || 'id' in data) {
    return [data as ProbeAgentRecord];
  }

  return Object.entries(data)
    .filter(([, value]) => isRecord(value))
    .map(([name, value]) => ({ name, ...(value as Record<string, unknown>) }));
}

export function matchesAgent(record: ProbeAgentRecord, agentName: string): boolean {
  return [record.name, record.agent, record.id].some((value) => value === agentName);
}

export function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function shouldRetryProbe(error: string): boolean {
  return isOfflineProbeError(error);
}

export function isProbeTimeout(error: string): boolean {
  return error.includes(AGENT_PROBE_TIMEOUT_MESSAGE);
}

export function stripProbeUiState(probe: AgentProbeResult): AgentProbeResult {
  const { refreshing: _refreshing, degradedMessage: _degradedMessage, ...rest } = probe;
  return rest;
}

function formatProbeAge(probedAt: number, now = Date.now()): string {
  const elapsedMs = Math.max(0, now - probedAt);
  const elapsedSeconds = Math.round(elapsedMs / 1_000);
  if (elapsedSeconds < 60) {
    return `${elapsedSeconds}s`;
  }

  const elapsedMinutes = Math.round(elapsedSeconds / 60);
  if (elapsedMinutes < 60) {
    return `${elapsedMinutes}m`;
  }

  const elapsedHours = Math.round(elapsedMinutes / 60);
  return `${elapsedHours}h`;
}

export function refreshingMessage(probedAt: number): string {
  return `Refreshing live capabilities. Cached options from ${formatProbeAge(probedAt)} ago stay available until the probe finishes.`;
}

export function cachedFailureMessage(probedAt: number, error: string): string {
  const age = formatProbeAge(probedAt);
  if (error.toLowerCase().includes('timed out')) {
    return `Live refresh timed out. Showing cached capabilities from ${age} ago.`;
  }

  return `Live probe is unavailable right now. Showing cached capabilities from ${age} ago.`;
}

export async function getConfigWithSoftTimeout(
  gatewayExecute: ReturnType<typeof usePipelineMcp>['gatewayExecute'],
): Promise<unknown> {
  let timeoutHandle: ReturnType<typeof setTimeout> | null = null;

  try {
    return await Promise.race([
      gatewayExecute<unknown>('maenifold.system', 'GetConfig', {}),
      new Promise<never>((_, reject) => {
        timeoutHandle = setTimeout(() => {
          reject(new Error(AGENT_PROBE_TIMEOUT_MESSAGE));
        }, AGENT_PROBE_SOFT_TIMEOUT_MS);
      }),
    ]);
  } finally {
    if (timeoutHandle) {
      clearTimeout(timeoutHandle);
    }
  }
}
