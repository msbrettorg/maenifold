import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { createMcpClientOps, type SdkClient } from '../../composables/mcpClientOps';
import type { ToolCallResult } from '../../composables/mcpTransport';
import { useMcpToolDiscoveryStore } from '../mcp-tool-discovery';

const mcpClientMock = vi.hoisted(() => ({
  connected: { value: true },
  connect: vi.fn(),
  callTool: vi.fn(),
  callToolRaw: vi.fn(),
})) as {
  connected: { value: boolean };
  connect: ReturnType<typeof vi.fn>;
  callTool: any;
  callToolRaw: any;
};

vi.mock('../../composables/useMcpClient', () => ({
  useMcpClient: () => mcpClientMock,
}));

function toolText(payload: unknown): ToolCallResult {
  return {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  };
}

function createRoutingClient(
  callToolImpl: (request: { name: string; arguments?: Record<string, unknown> }) => Promise<ToolCallResult>,
): SdkClient & { callTool: ReturnType<typeof vi.fn> } {
  return {
    callTool: vi.fn(callToolImpl),
  } as unknown as SdkClient & { callTool: ReturnType<typeof vi.fn> };
}

describe('useMcpToolDiscoveryStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mcpClientMock.connected.value = true;
    mcpClientMock.connect.mockReset();
    mcpClientMock.callTool = vi.fn();
    mcpClientMock.callToolRaw = vi.fn();
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
    const transportClient = createRoutingClient(async ({ name, arguments: args = {} }) => {
      if (name === 'DiscoverMcpTools') {
        expect(args).toEqual({});
        return toolText([
          { server: 'filesystem', type: 'stdio', status: 'ok', tools: [{ name: 'filesystem.ReadFile', inputSchema: {} }] },
        ]);
      }

      if (name === 'GatewayIndex') {
        throw new Error('GatewayIndex should not run for allowlisted passthrough tools');
      }

      if (name === 'GatewayExecute') {
        throw new Error('GatewayExecute should not run for DiscoverMcpTools passthrough calls');
      }

      throw new Error(`Unexpected transport call for ${name}`);
    });

    const ops = createMcpClientOps({
      ensureConnectedSession: async () => transportClient,
      reconnectSession: async () => undefined,
      setError: vi.fn(),
    });
    mcpClientMock.callTool = ops.callTool;
    mcpClientMock.callToolRaw = vi.fn(ops.callToolRaw);

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
    expect(transportClient.callTool).toHaveBeenCalledTimes(1);
    expect(transportClient.callTool).toHaveBeenCalledWith({
      name: 'DiscoverMcpTools',
      arguments: {},
    });
    expect(transportClient.callTool.mock.calls.some(([request]) => request.name === 'GatewayIndex')).toBe(false);
    expect(transportClient.callTool.mock.calls.some(([request]) => request.name === 'GatewayExecute')).toBe(false);
    expect(store.error).toBeNull();
  });
});
