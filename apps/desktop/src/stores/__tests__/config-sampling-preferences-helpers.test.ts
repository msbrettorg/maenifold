// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentsConfig } from '../../lib/types';

const _samplingAgentTypeCheck: AgentsConfig = { agents: {}, samplingAgent: 'copilot' };
void _samplingAgentTypeCheck;

type SamplingPreference = { alwaysAllow: boolean; modelOverride?: string };
type SamplingHelperConfig = AgentsConfig & {
  samplingAgent?: string;
  samplingPreferences?: {
    perServer?: Record<string, SamplingPreference>;
  };
};

type ConfigStoreWithSamplingHelpers = {
  loadConfig(): Promise<void>;
  config: SamplingHelperConfig;
  setSamplingPreference(serverName: string, pref: SamplingPreference): Promise<void>;
  clearSamplingPreference(serverName: string): Promise<void>;
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
  return { configRead, configWrite, store: useConfigStore() as unknown as ConfigStoreWithSamplingHelpers };
}

function expectSetHelper(store: ConfigStoreWithSamplingHelpers) {
  expect(
    typeof store.setSamplingPreference,
    'useConfigStore must return callable setSamplingPreference(serverName, pref)',
  ).toBe('function');
  return store.setSamplingPreference.bind(store);
}

function expectClearHelper(store: ConfigStoreWithSamplingHelpers) {
  expect(
    typeof store.clearSamplingPreference,
    'useConfigStore must return callable clearSamplingPreference(serverName)',
  ).toBe('function');
  return store.clearSamplingPreference.bind(store);
}

function lastSavedConfig(configWrite: ReturnType<typeof vi.fn>): SamplingHelperConfig {
  const lastCall = configWrite.mock.calls[configWrite.mock.calls.length - 1];
  expect(lastCall, 'helper must persist through the existing configWrite IPC path').toBeDefined();
  return JSON.parse(lastCall![0]) as SamplingHelperConfig;
}

describe('useConfigStore sampling preference helpers', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'electronAPI');
  });

  it('returns callable helpers for Vue component consumers', async () => {
    const { store } = await setupConfigStore();

    expect(typeof store.setSamplingPreference).toBe('function');
    expect(typeof store.clearSamplingPreference).toBe('function');
  });

  it('sets the first per-server preference when samplingPreferences is absent', async () => {
    const { configWrite, store } = await setupConfigStore('{"mcpServers":[],"agents":{}}');
    await store.loadConfig();

    await expectSetHelper(store)('server-a', { alwaysAllow: true });

    const saved = lastSavedConfig(configWrite);
    expect(saved.samplingPreferences?.perServer?.['server-a']).toEqual({ alwaysAllow: true });
  });

  it('preserves sibling per-server preferences when replacing one server', async () => {
    const prefA: SamplingPreference = { alwaysAllow: true };
    const prefB: SamplingPreference = { alwaysAllow: false, modelOverride: 'gpt-4o' };
    const newPrefA: SamplingPreference = { alwaysAllow: false, modelOverride: 'claude' };
    const { configWrite, store } = await setupConfigStore(JSON.stringify({
      mcpServers: [],
      agents: {},
      samplingPreferences: { perServer: { a: prefA, b: prefB } },
    }));
    await store.loadConfig();

    await expectSetHelper(store)('a', newPrefA);

    const saved = lastSavedConfig(configWrite);
    expect(saved.samplingPreferences?.perServer?.a).toEqual(newPrefA);
    expect(saved.samplingPreferences?.perServer?.b).toEqual(prefB);
  });

  it('preserves all entries across three sequential set calls for distinct servers', async () => {
    const { configWrite, store } = await setupConfigStore();
    await store.loadConfig();

    const setSamplingPreference = expectSetHelper(store);
    await setSamplingPreference('a', { alwaysAllow: true });
    await setSamplingPreference('b', { alwaysAllow: false, modelOverride: 'gpt-4o' });
    await setSamplingPreference('c', { alwaysAllow: true, modelOverride: 'claude' });

    const saved = lastSavedConfig(configWrite);
    expect(saved.samplingPreferences?.perServer).toEqual({
      a: { alwaysAllow: true },
      b: { alwaysAllow: false, modelOverride: 'gpt-4o' },
      c: { alwaysAllow: true, modelOverride: 'claude' },
    });
  });

  it('clearSamplingPreference removes only the requested server', async () => {
    const { configWrite, store } = await setupConfigStore(JSON.stringify({
      mcpServers: [],
      agents: {},
      samplingPreferences: {
        perServer: {
          a: { alwaysAllow: true },
          b: { alwaysAllow: false, modelOverride: 'gpt-4o' },
          c: { alwaysAllow: true, modelOverride: 'claude' },
        },
      },
    }));
    await store.loadConfig();

    await expectClearHelper(store)('a');

    const saved = lastSavedConfig(configWrite);
    expect(saved.samplingPreferences?.perServer).toEqual({
      b: { alwaysAllow: false, modelOverride: 'gpt-4o' },
      c: { alwaysAllow: true, modelOverride: 'claude' },
    });
  });

  it('preserves top-level samplingAgent through parse, helper write, serialize, and reload', async () => {
    const { configRead, configWrite, store } = await setupConfigStore(JSON.stringify({
      mcpServers: [],
      agents: {},
      samplingAgent: 'copilot',
      samplingPreferences: { perServer: { a: { alwaysAllow: false } } },
    }));
    await store.loadConfig();
    expect(
      store.config.samplingAgent,
      'loadConfig must parse top-level samplingAgent before helper writes run',
    ).toBe('copilot');

    await expectSetHelper(store)('foo', { alwaysAllow: true });

    const saved = lastSavedConfig(configWrite);
    expect(saved.samplingAgent, 'helper save must serialize top-level samplingAgent').toBe('copilot');
    expect(saved.samplingPreferences?.perServer?.a).toEqual({ alwaysAllow: false });
    expect(saved.samplingPreferences?.perServer?.foo).toEqual({ alwaysAllow: true });
    configRead.mockResolvedValue({ path: '/test/maenifold.json', content: configWrite.mock.calls[0][0], exists: true });
    await store.loadConfig();
    expect(store.config.samplingAgent, 'reload after helper save must preserve samplingAgent').toBe('copilot');
  });

  it('preserves top-level mcpServers, agents, and settings when setting a server preference', async () => {
    const settings = {
      logRotation: 'daily',
      debounceMs: 50,
      autoSync: true,
      syncLogging: false,
      sessionCleanup: true,
      sqliteBusyTimeoutMs: 5000,
      syncLeaseMs: 10000,
      watcherBufferSize: 128,
      incrementalOptimizeEvery: 25,
      incrementalVacuumMinutes: 60,
      sessionAbandonMinutes: null,
      snippetLength: 240,
      syncRole: null,
      decay: {
        graceDaysSequential: 1,
        graceDaysWorkflows: 2,
        graceDaysDefault: 3,
        halfLifeDays: 30,
        function: 'exponential',
      },
      community: {
        louvainGamma: 1,
        minSharedNeighbors: 2,
        minOverlap: 0.25,
        maxSiblings: 8,
        debounceMs: 100,
        nmiThreshold: 0.7,
        recomputeFileThreshold: null,
      },
      vector: {
        embeddingLogs: false,
        vectorLogs: false,
        disableVector: false,
      },
      debug: {
        trace: false,
        pipelineDriver: null,
      },
    };
    const { configWrite, store } = await setupConfigStore(JSON.stringify({
      mcpServers: [{ type: 'stdio', name: 'server-a', command: 'npx' }],
      agents: { copilot: { command: 'copilot', env: { NODE_ENV: 'test' } } },
      settings,
    }));
    await store.loadConfig();

    await expectSetHelper(store)('foo', { alwaysAllow: true });

    const saved = lastSavedConfig(configWrite);
    expect(saved.mcpServers).toEqual([{ type: 'stdio', name: 'server-a', command: 'npx' }]);
    expect(saved.agents).toEqual({
      copilot: {
        command: 'copilot',
        env: { NODE_ENV: 'test' },
      },
    });
    expect(store.config.agents).toEqual({ copilot: { command: 'copilot', env: { NODE_ENV: 'test' } } });
    expect(saved.settings).toEqual(settings);
    expect(saved.samplingPreferences?.perServer?.foo).toEqual({ alwaysAllow: true });
  });
});
