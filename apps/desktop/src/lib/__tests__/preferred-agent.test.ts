import { beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { PREFS_STORE_PATH } from '../constants';
import { storageKey } from '../storage-key';
import { resolveAgents } from '../pipeline-template-runtime';
import { readLastUsedAgent, resolvePreferredAgent, writeLastUsedAgent } from '../preferred-agent';

describe('preferred agent helpers', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('stores and reads the last used agent preference', async () => {
    writeLastUsedAgent('Copilot');

    expect(readLastUsedAgent()).toBe('Copilot');
    await nextTick();
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

  it('t301_29 defaults to Copilot when no stored preference and Copilot is available', () => {
    expect(resolvePreferredAgent(['Codex', 'Copilot', 'OpenCode'])).toBe('Copilot');
  });

  it('t301_29 falls back to first agent when no stored preference and Copilot is unavailable', () => {
    expect(resolvePreferredAgent(['OpenCode', 'Codex'])).toBe('OpenCode');
  });

  it('t301_29 stored preference still wins over Copilot default', () => {
    writeLastUsedAgent('OpenCode');

    expect(resolvePreferredAgent(['OpenCode', 'Copilot', 'Codex'])).toBe('OpenCode');
  });

  it('t301_29 stale stored preference falls back to Copilot when present, otherwise first agent', () => {
    writeLastUsedAgent('Droid');

    expect(resolvePreferredAgent(['Codex', 'Copilot'])).toBe('Copilot');
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
