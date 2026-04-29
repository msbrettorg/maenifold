import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useMcpToolDiscoveryStore } from '../mcp-tool-discovery';

const mcpClientMock = vi.hoisted(() => ({
  connected: { value: true },
  connect: vi.fn(),
  callTool: vi.fn(),
}));

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => mcpClientMock,
}));

describe('useMcpToolDiscoveryStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mcpClientMock.connected.value = true;
    mcpClientMock.connect.mockReset();
    mcpClientMock.callTool.mockReset();
  });

  it('derives ready, auth, stale, and unavailable server health from discovery results', async () => {
    mcpClientMock.callTool
      .mockResolvedValueOnce([
        {
          server: 'filesystem',
          type: 'stdio',
          status: 'ok',
          tools: [
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
          ],
        },
        { server: 'msx-mcp', type: 'stdio', status: 'ok', tools: [{ name: 'msx-mcp.list_accounts', inputSchema: {} }] },
      ])
      .mockResolvedValueOnce([
        { server: 'filesystem', type: 'stdio', status: 'error', error: 'connection timed out', tools: [] },
        { server: 'msx-mcp', type: 'stdio', status: 'error', error: 'login required', tools: [] },
        { server: 'broken', type: 'stdio', status: 'error', error: 'socket refused', tools: [] },
      ]);

    const store = useMcpToolDiscoveryStore();
    await store.refreshTools();
    await store.refreshTools();

    expect(store.hasStaleCacheByServer).toEqual({
      filesystem: true,
      'msx-mcp': true,
      broken: false,
    });
    expect(store.serverHealth).toEqual({
      filesystem: 'stale',
      'msx-mcp': 'auth_required',
      broken: 'unavailable',
    });
    expect(store.serverResults[0]?.tools[0]?.name).toBe('filesystem.ReadFile');
    expect(store.serverResults[0]?.tools[0]?.appUi).toEqual({
      resourceUri: 'ui://filesystem/read-file.html',
      csp: "default-src 'self'",
      permissions: ['camera'],
    });
  });

  it('loads DiscoverMcpTools through the MCP client without the direct fallback routing error', async () => {
    mcpClientMock.callTool.mockResolvedValueOnce([
      { server: 'filesystem', type: 'stdio', status: 'ok', tools: [{ name: 'filesystem.ReadFile', inputSchema: {} }] },
    ]);

    const store = useMcpToolDiscoveryStore();

    await expect(store.refreshTools()).resolves.toEqual([
      {
        server: 'filesystem',
        type: 'stdio',
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
    expect(mcpClientMock.callTool).toHaveBeenCalledWith('DiscoverMcpTools', {});
    expect(store.error).toBeNull();
  });
});
