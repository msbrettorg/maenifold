import { beforeEach, describe, expect, it } from 'vitest';
import { PREFS_STORE_PATH } from '../constants';
import { storageKey } from '../local-storage';
import { resolveAgents } from '../pipeline-template-runtime';
import { readLastUsedAgent, resolvePreferredAgent, writeLastUsedAgent } from '../preferred-agent';

describe('preferred agent helpers', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('stores and reads the last used agent preference', () => {
    writeLastUsedAgent('Copilot');

    expect(readLastUsedAgent()).toBe('Copilot');
    expect(window.localStorage.getItem(storageKey(PREFS_STORE_PATH, 'lastUsedAgent'))).toBe('"Copilot"');
  });

  it('prefers the stored agent when it is still configured', () => {
    writeLastUsedAgent('Copilot');

    expect(resolvePreferredAgent(['OpenCode', 'Copilot', 'Codex'])).toBe('Copilot');
  });

  it('falls back to the first configured agent when the stored agent is unavailable', () => {
    writeLastUsedAgent('Copilot');

    expect(resolvePreferredAgent(['OpenCode', 'Codex'])).toBe('OpenCode');
  });

  it('lets template agent resolution honor the preferred agent while keeping a distinct critic', () => {
    const resolved = resolveAgents(['OpenCode', 'Codex', 'Copilot'], 'Copilot');

    expect(resolved).toEqual({
      primaryAgent: 'Copilot',
      criticAgent: 'OpenCode',
    });
  });
});
