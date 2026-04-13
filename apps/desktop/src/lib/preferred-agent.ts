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

export function resolvePreferredAgent(agentNames: readonly string[]): string {
  const preferredAgent = readLastUsedAgent();
  if (preferredAgent && agentNames.includes(preferredAgent)) {
    return preferredAgent;
  }

  return normalizeAgentName(agentNames[0]);
}
