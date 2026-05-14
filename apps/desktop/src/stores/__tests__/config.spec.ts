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

describe('config store agents serializer contract', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'electronAPI');
  });

  it('serializeConfig_emits_agents_as_object_keyed_by_name', async () => {
    const { store, configWrite } = await setupConfigStore();

    await store.addAgent('claude', { command: '/usr/local/bin/claude' });

    const lastCall = configWrite.mock.calls[configWrite.mock.calls.length - 1];
    const parsed = JSON.parse(lastCall?.[0] ?? '{}') as AgentsConfig;
    expect(Array.isArray(parsed.agents)).toBe(false);
    expect(Object.getPrototypeOf(parsed.agents)).toBe(Object.prototype);
    expect(parsed.agents.claude.command).toBe('/usr/local/bin/claude');
  });

  it('serializeConfig_round_trips_through_parseAgentsConfig', async () => {
    const { store, configWrite } = await setupConfigStore();

    await store.addAgent('gpt5', { command: 'gpt5 --fast' });
    await store.addAgent('claude', { command: 'claude --model opus' });

    const lastCall = configWrite.mock.calls[configWrite.mock.calls.length - 1];
    const serialized = lastCall?.[0] ?? '{}';
    const { parseAgentsConfig } = await import('../config');
    const parsed = parseAgentsConfig(serialized);

    expect(Object.keys(parsed.agents).sort()).toEqual(['claude', 'gpt5']);
    expect(parsed.agents.gpt5.command).toBe('gpt5 --fast');
    expect(parsed.agents.claude.command).toBe('claude --model opus');
  });

  it('agents_roundtrip_via_save_and_load', async () => {
    const first = await setupConfigStore();
    await first.store.addAgent('gpt5', { command: 'gpt5 --fast' });
    await first.store.addAgent('claude', { command: 'claude --model opus' });
    const serialized = first.configWrite.mock.calls[first.configWrite.mock.calls.length - 1]?.[0] ?? '{}';

    const second = await setupConfigStore(serialized);

    expect(Object.keys(second.store.config.agents).sort()).toEqual(['claude', 'gpt5']);
    expect(second.store.config.agents.gpt5.command).toBe('gpt5 --fast');
    expect(second.store.config.agents.claude.command).toBe('claude --model opus');
    const onDisk = JSON.parse(serialized) as AgentsConfig;
    expect(Array.isArray(onDisk.agents)).toBe(false);
  });

  it('parseAgentsConfig_accepts_legacy_array_agent_shape_for_migration', async () => {
    // Old on-disk shape (pre-T-307.4) — agents written as array.
    const legacy = JSON.stringify({
      mcpServers: [],
      agents: [
        { name: 'codex', command: '/path/to/codex' },
        { name: 'gpt5', command: 'gpt5 --fast' },
      ],
    });

    const { parseAgentsConfig } = await import('../config');
    const parsed = parseAgentsConfig(legacy);

    // Migration path: array entries are converted to object form keyed by name.
    expect(Array.isArray(parsed.agents)).toBe(false);
    expect(Object.keys(parsed.agents).sort()).toEqual(['codex', 'gpt5']);
    expect(parsed.agents.codex.command).toBe('/path/to/codex');
    expect(parsed.agents.gpt5.command).toBe('gpt5 --fast');
  });
});
