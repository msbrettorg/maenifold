// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentsConfig, McpServersMap } from '../../lib/types';

async function setupConfigStore(content = '{"mcpServers":{},"agents":{}}') {
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

  it('serializeConfig_does_not_split_command_into_args_field', async () => {
    const { store, configWrite } = await setupConfigStore();

    // User's actual config: copilot agent with --acp flag (verified in ~/maenifold/maenifold.json).
    await store.addAgent('copilot', { command: '/path/to/copilot --acp' });

    const lastCall = configWrite.mock.calls[configWrite.mock.calls.length - 1];
    const parsed = JSON.parse(lastCall?.[0] ?? '{}') as AgentsConfig;

    // The on-disk shape must match Rust AgentEntry: command is the FULL joined string,
    // no separate args field (Rust struct has no args field — would silently drop them).
    expect(parsed.agents.copilot.command).toBe('/path/to/copilot --acp');
    expect((parsed.agents.copilot as unknown as Record<string, unknown>).args).toBeUndefined();
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
      mcpServers: {},
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

  it('mcp_servers_schema_hydrates_object_keyed_servers_and_exposes_named_entries', async () => {
    const content = JSON.stringify({
      mcpServers: {
        workiq: {
          command: '/usr/bin/env',
          args: ['agency', 'mcp', 'workiq'],
          gateway: true,
        },
        learn: {
          url: 'https://learn.microsoft.com/api/mcp',
          gateway: true,
        },
      } satisfies McpServersMap,
      agents: {},
    });

    const { store } = await setupConfigStore(content);
    const entries = Object.entries(store.config.mcpServers ?? {});

    expect(entries.map(([name]) => name)).toEqual(['workiq', 'learn']);
    expect(entries[0]).toEqual([
      'workiq',
      {
        command: '/usr/bin/env',
        args: ['agency', 'mcp', 'workiq'],
        gateway: true,
      },
    ]);
    expect(entries[1]).toEqual([
      'learn',
      {
        url: 'https://learn.microsoft.com/api/mcp',
        gateway: true,
      },
    ]);
  });

  it('mcp_servers_schema_add_mcp_server_rejects_duplicate_name', async () => {
    const { store, configWrite } = await setupConfigStore(
      JSON.stringify({
        mcpServers: {
          workiq: {
            command: '/usr/bin/env',
            args: ['agency', 'mcp', 'workiq'],
          },
        } satisfies McpServersMap,
        agents: {},
      }),
    );

    await expect(
      store.addMcpServer('workiq', {
        url: 'https://example.invalid/mcp',
      }),
    ).rejects.toThrow(/duplicate|exists|unique/i);
    expect(configWrite).not.toHaveBeenCalled();
  });
});
