// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentsConfig } from '../../lib/types';

async function setupConfigStore(content = '{"mcpServers":[],"agents":{}}') {
  vi.resetModules();
  setActivePinia(createPinia());
  const configRead = vi.fn().mockResolvedValue({ path: '/test/maenifold.json', content, exists: true });
  const configWrite = vi.fn().mockResolvedValue({ path: '/test/maenifold.json' });
  Object.defineProperty(window, 'electronAPI', {
    value: {
      configRead,
      configWrite,
      configPath: vi.fn().mockResolvedValue('/test/maenifold.json'),
      onConfigChanged: vi.fn().mockReturnValue(vi.fn()),
    },
    configurable: true,
  });
  const { useConfigStore } = await import('../config');
  const store = useConfigStore();
  await store.loadConfig();
  return { store, configWrite };
}

describe('config store tool loop budget round trip', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'electronAPI');
  });

  it('round-trips default and per-server sampling tool loop budgets through store helpers', async () => {
    const { store, configWrite } = await setupConfigStore();

    await store.setDefaultToolLoopBudget(25);

    expect(store.config.samplingPreferences?.defaultToolLoopBudget).toBe(25);

    await store.setSamplingPreference('foo', { alwaysAllow: true, toolLoopBudget: 7 });

    expect(store.config.samplingPreferences?.perServer?.foo?.toolLoopBudget).toBe(7);
    const lastCall = configWrite.mock.calls[configWrite.mock.calls.length - 1];
    const serialized = JSON.parse(lastCall?.[0] ?? '{}') as AgentsConfig;
    expect(serialized.samplingPreferences?.defaultToolLoopBudget).toBe(25);
    expect(serialized.samplingPreferences?.perServer?.foo?.toolLoopBudget).toBe(7);
  });
});
