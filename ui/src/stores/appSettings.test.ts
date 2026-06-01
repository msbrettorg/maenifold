import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

vi.mock('../lib/mcp/client', async (importActual) => {
  const actual = await importActual<typeof import('../lib/mcp/client')>();
  return { ...actual, mcpHttpClient: { callTool: vi.fn() } };
});

import { mcpHttpClient } from '../lib/mcp/client';
import { useAppSettingsStore } from './appSettings';

function toolText(value: unknown) {
  return {
    content: [{ type: 'text', text: JSON.stringify(value) }],
  };
}

const callToolMock = vi.mocked(mcpHttpClient.callTool);

beforeEach(() => {
  setActivePinia(createPinia());
  callToolMock.mockReset();
});

describe('useAppSettingsStore', () => {
  it('starts each fresh store with empty state', () => {
    const store = useAppSettingsStore();

    expect(store.schema).toBeNull();
    expect(store.values).toBeNull();
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('fetchSchema calls GatewayExecute/GetSettingsSchema and populates schema', async () => {
    const store = useAppSettingsStore();
    const nextSchema = {
      type: 'object',
      properties: {
        decay: {
          type: 'object',
        },
      },
    };

    callToolMock.mockResolvedValueOnce(toolText(nextSchema));

    await store.fetchSchema();

    expect(callToolMock).toHaveBeenCalledTimes(1);
    expect(callToolMock).toHaveBeenCalledWith('GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetSettingsSchema',
      parameters: {},
    });
    expect(store.schema).toEqual(nextSchema);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('fetchConfig calls GatewayExecute/GetConfig and stores config.settings as values', async () => {
    const store = useAppSettingsStore();
    const settings = {
      decay: {
        halfLifeDays: 45,
      },
      debounceMs: 200,
    };

    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {
          local: {
            command: 'maenifold',
          },
        },
        settings,
      })
    );

    await store.fetchConfig();

    expect(callToolMock).toHaveBeenCalledTimes(1);
    expect(callToolMock).toHaveBeenCalledWith('GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.values).toEqual(settings);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('setLeaf sends SetSettings with a minimal nested patch', async () => {
    const store = useAppSettingsStore();

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {},
        settings: {
          decay: {
            halfLifeDays: 45,
          },
        },
      })
    );

    await store.setLeaf(['decay', 'halfLifeDays'], 45);

    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'SetSettings',
      parameters: {
        settings: {
          decay: {
            halfLifeDays: 45,
          },
        },
      },
    });
  });

  it('setLeaf re-fetches config after a successful save', async () => {
    const store = useAppSettingsStore();
    const refreshedSettings = {
      debounceMs: 200,
    };

    callToolMock.mockResolvedValueOnce(toolText({ ok: true }));
    callToolMock.mockResolvedValueOnce(
      toolText({
        agents: {},
        settings: refreshedSettings,
      })
    );

    await store.setLeaf(['debounceMs'], 200);

    expect(callToolMock).toHaveBeenCalledTimes(2);
    expect(callToolMock).toHaveBeenNthCalledWith(2, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(store.values).toEqual(refreshedSettings);
    expect(store.loading).toBe(false);
  });

  it('fetchSchema surfaces errors and clears loading on failure', async () => {
    const store = useAppSettingsStore();

    callToolMock.mockRejectedValueOnce(new Error('schema boom'));

    await expect(store.fetchSchema()).rejects.toThrow('schema boom');

    expect(store.error).toBe('schema boom');
    expect(store.loading).toBe(false);
  });
});
