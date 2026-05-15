import { PREFS_STORE_PATH } from './constants';
import { readStorageValue, writeStorageValue } from './local-storage';

export const LAST_USED_AGENT_KEY = 'lastUsedAgent';

function normalizeAgentName(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

export function readLastUsedAgent(): string | null {
  const agentName = normalizeAgentName(
    readStorageValue<string>(PREFS_STORE_PATH, LAST_USED_AGENT_KEY),
  );
  return agentName || null;
}

export function writeLastUsedAgent(agentName: string): void {
  const normalized = normalizeAgentName(agentName);
  if (!normalized) {
    return;
  }

  writeStorageValue(PREFS_STORE_PATH, LAST_USED_AGENT_KEY, normalized);
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
