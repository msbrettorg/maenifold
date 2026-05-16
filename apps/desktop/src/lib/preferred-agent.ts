import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { PREFS_STORE_PATH } from './constants';
import { storageKey } from './storage-key';

export const LAST_USED_AGENT_KEY = 'lastUsedAgent';
const lastUsedAgent = useLocalStorage<string>(
  storageKey(PREFS_STORE_PATH, LAST_USED_AGENT_KEY),
  '',
  { serializer: StorageSerializers.object },
);

function normalizeAgentName(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

export function readLastUsedAgent(): string | null {
  const agentName = normalizeAgentName(lastUsedAgent.value);
  return agentName || null;
}

export function writeLastUsedAgent(agentName: string): void {
  const normalized = normalizeAgentName(agentName);
  if (!normalized) {
    return;
  }

  lastUsedAgent.value = normalized;
}

/**
 * Well-known agent that works out-of-box without env-var configuration.
 * Copilot uses GitHub's built-in `gh` CLI auth, so no envFromFile is needed.
 * See docs/DEVELOPMENT.md for the agent configuration patterns.
 */
const PREFERRED_DEFAULT_AGENT = 'Copilot';

export function resolvePreferredAgent(agentNames: readonly string[]): string {
  const preferredAgent = readLastUsedAgent();
  if (preferredAgent && agentNames.includes(preferredAgent)) {
    return preferredAgent;
  }

  if (agentNames.includes(PREFERRED_DEFAULT_AGENT)) {
    return PREFERRED_DEFAULT_AGENT;
  }

  return normalizeAgentName(agentNames[0]);
}
