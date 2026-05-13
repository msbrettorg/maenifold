// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentsConfig } from '../../lib/types';

type SamplingPreferencesConfig = AgentsConfig & {
  samplingPreferences?: {
    perServer?: Record<string, { alwaysAllow: boolean; modelOverride?: string }>;
  };
};

type ConfigStoreWithWriteConfig = {
  writeConfig(updates: Partial<SamplingPreferencesConfig>): Promise<void>;
};

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
  return { configRead, configWrite, store: useConfigStore() };
}

describe('config samplingPreferences contract', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'electronAPI');
  });

  it('parseAgentsConfig_preserves_samplingPreferences_perServer', async () => {
    const { store } = await setupConfigStore(JSON.stringify({
      mcpServers: [],
      agents: {},
      samplingPreferences: { perServer: { foo: { alwaysAllow: true } } },
    }));

    await store.loadConfig();

    const parsed = store.config as SamplingPreferencesConfig;
    expect(parsed.samplingPreferences?.perServer?.foo?.alwaysAllow).toBe(true);
  });

  it('parseAgentsConfig_handles_missing_samplingPreferences_as_undefined', async () => {
    const { store } = await setupConfigStore('{"mcpServers":[],"agents":{}}');

    await store.loadConfig();

    const parsed = store.config as SamplingPreferencesConfig;
    expect(parsed.samplingPreferences).toBeUndefined();
  });

  it('serializeConfig_emits_samplingPreferences_when_present', async () => {
    const { configWrite, store } = await setupConfigStore();
    store.config = {
      mcpServers: [],
      agents: {},
      samplingPreferences: { perServer: { a: { alwaysAllow: false, modelOverride: 'gpt-4o' } } },
    } as SamplingPreferencesConfig;

    await store.save();

    const serialized = JSON.parse(configWrite.mock.calls[0][0]) as SamplingPreferencesConfig;
    expect(serialized.samplingPreferences?.perServer?.a?.modelOverride).toBe('gpt-4o');
  });

  it('serializeConfig_omits_samplingPreferences_key_when_undefined', async () => {
    const { configWrite, store } = await setupConfigStore();
    store.config = { mcpServers: [], agents: {} };

    await store.save();

    const serialized = JSON.parse(configWrite.mock.calls[0][0]) as SamplingPreferencesConfig;
    expect(serialized.samplingPreferences).toBeUndefined();
  });

  it('useConfigStore_writeConfig_partial_merges_samplingPreferences_without_disturbing_mcpServers', async () => {
    const { configWrite, store } = await setupConfigStore(JSON.stringify({
      mcpServers: [{ type: 'stdio', name: 'a', command: 'x' }],
      agents: {},
    }));
    await store.loadConfig();

    await (store as unknown as ConfigStoreWithWriteConfig).writeConfig({
      samplingPreferences: { perServer: { a: { alwaysAllow: true } } },
    });

    const saved = JSON.parse(configWrite.mock.calls[0][0]) as SamplingPreferencesConfig;
    expect(saved.mcpServers).toEqual([{ type: 'stdio', name: 'a', command: 'x' }]);
    expect(saved.samplingPreferences?.perServer?.a?.alwaysAllow).toBe(true);
    expect((store.config as SamplingPreferencesConfig).mcpServers).toEqual(saved.mcpServers);
  });

  it('roundtrip_preserves_unrelated_blocks', async () => {
    const initial = {
      mcpServers: [
        { type: 'stdio', name: 'one', command: 'a' },
        { type: 'stdio', name: 'two', command: 'b' },
        { type: 'http', name: 'three', url: 'https://example.test/mcp' },
      ],
      agents: { alpha: { command: 'agent-a' }, beta: { command: 'agent-b' } },
      samplingPreferences: { perServer: { one: { alwaysAllow: false } } },
    };
    const { configRead, configWrite, store } = await setupConfigStore(JSON.stringify(initial));
    await store.loadConfig();

    await (store as unknown as ConfigStoreWithWriteConfig).writeConfig({
      samplingPreferences: { perServer: { one: { alwaysAllow: true } } },
    });
    configRead.mockResolvedValue({ path: '/test/maenifold.json', content: configWrite.mock.calls[0][0], exists: true });
    await store.loadConfig();

    expect(store.config.mcpServers).toEqual(initial.mcpServers);
    expect(store.config.agents).toEqual(initial.agents);
    expect((store.config as SamplingPreferencesConfig).samplingPreferences?.perServer?.one?.alwaysAllow).toBe(true);
  });
});
