import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useMcpToolDiscoveryStore } from '../mcp-tool-discovery';

const mcpClientMock = vi.hoisted(() => ({
  connected: { value: true },
  connect: vi.fn(),
  listTools: vi.fn(),
})) as {
  connected: { value: boolean };
  connect: ReturnType<typeof vi.fn>;
  listTools: any;
};

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => mcpClientMock,
}));

describe('useMcpToolDiscoveryStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mcpClientMock.connected.value = true;
    mcpClientMock.connect.mockReset();
    mcpClientMock.listTools = vi.fn();
  });

  it('groups tools by inferred server name and preserves stale cache on listTools failures', async () => {
    mcpClientMock.listTools
      .mockResolvedValueOnce([
        {
          name: 'filesystem.ReadFile',
          inputSchema: {},
          _meta: {
            ui: {
              resourceUri: 'ui://filesystem/read-file.html',
              csp: "default-src 'self'",
              permissions: ['camera', 42],
            },
          },
        },
        { name: 'msx-mcp.list_accounts', inputSchema: {} },
      ])
      .mockRejectedValueOnce(new Error('login required'));

    const store = useMcpToolDiscoveryStore();
    await store.refreshTools();
    await store.refreshTools();

    expect(store.hasStaleCacheByServer).toEqual({
      filesystem: true,
      'msx-mcp': true,
    });
    expect(store.serverHealth).toEqual({
      filesystem: 'stale',
      'msx-mcp': 'auth_required',
    });
    expect(store.serverResults[0]?.tools[0]?.name).toBe('filesystem.ReadFile');
    expect(store.serverResults[0]?.tools[0]?.appUi).toEqual({
      resourceUri: 'ui://filesystem/read-file.html',
      csp: "default-src 'self'",
      permissions: ['camera'],
    });
  });

  it('loads tools through the SDK listTools request and maps them into discovery results', async () => {
    mcpClientMock.listTools.mockResolvedValue([
      { name: 'filesystem.ReadFile', inputSchema: {} },
    ]);

    const store = useMcpToolDiscoveryStore();

    await expect(store.refreshTools()).resolves.toEqual([
      {
        server: 'filesystem',
        type: 'session',
        status: 'ok',
        error: null,
        tools: [{
          name: 'filesystem.ReadFile',
          description: null,
          inputSchema: {},
          appUi: null,
        }],
      },
    ]);
    expect(mcpClientMock.listTools).toHaveBeenCalledTimes(1);
    expect(store.error).toBeNull();
  });
});
