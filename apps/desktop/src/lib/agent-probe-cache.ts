import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import {
  normalizeAgentProbeResult,
} from './agent-probe-normalizers';
import type { AgentProbeResult } from './agent-probe-types-defs';
import { PREFS_STORE_PATH } from './constants';
import { storageKey } from './storage-key';

export const AGENT_PROBE_CACHE_STORAGE_KEY = 'agentProbeCache';
const agentProbeCache = useLocalStorage<PersistedAgentProbeCache | null>(
  storageKey(PREFS_STORE_PATH, AGENT_PROBE_CACHE_STORAGE_KEY),
  null,
  { serializer: StorageSerializers.object },
);

interface PersistedAgentProbeCache {
  version: 1;
  probes: Record<string, unknown>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeAgentName(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function toCachedProbe(value: unknown): AgentProbeResult | null {
  if (!isRecord(value)) {
    return null;
  }

  const probedAt =
    typeof value.probedAt === 'number' && Number.isFinite(value.probedAt)
      ? value.probedAt
      : Date.now();
  const error = typeof value.error === 'string' ? value.error : undefined;

  return normalizeAgentProbeResult(value, { probedAt, error });
}

function stripTransientProbeState(probe: AgentProbeResult): AgentProbeResult {
  const {
    refreshing: _refreshing,
    degradedMessage: _degradedMessage,
    ...rest
  } = probe;
  return rest;
}

export function loadAgentProbeCache(): Map<string, AgentProbeResult> {
  const persisted = agentProbeCache.value;
  if (!persisted || persisted.version !== 1 || !isRecord(persisted.probes)) {
    return new Map();
  }

  return new Map(
    Object.entries(persisted.probes)
      .map(([agentName, probe]) => {
        const normalizedAgentName = normalizeAgentName(agentName);
        const normalizedProbe = toCachedProbe(probe);
        if (!normalizedAgentName || !normalizedProbe || normalizedProbe.state !== 'ready') {
          return null;
        }

        return [normalizedAgentName, normalizedProbe] as const;
      })
      .filter(
        (entry): entry is readonly [string, AgentProbeResult] => entry !== null,
      ),
  );
}

export function persistAgentProbeCache(
  probes: Map<string, AgentProbeResult>,
): void {
  const cachedProbes = Object.fromEntries(
    [...probes.entries()]
      .map(([agentName, probe]) => {
        const normalizedAgentName = normalizeAgentName(agentName);
        if (!normalizedAgentName || probe.state !== 'ready') {
          return null;
        }

        return [normalizedAgentName, stripTransientProbeState(probe)] as const;
      })
      .filter(
        (entry): entry is readonly [string, AgentProbeResult] => entry !== null,
      ),
  );

  agentProbeCache.value = {
    version: 1,
    probes: cachedProbes,
  } satisfies PersistedAgentProbeCache;
}

/** Test-only: clears the in-memory cache singleton so storage.clear() takes effect. */
export function __resetAgentProbeCacheForTests(): void {
  agentProbeCache.value = null;
}
