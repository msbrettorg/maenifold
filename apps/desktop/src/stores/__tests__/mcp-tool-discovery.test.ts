import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useMcpToolDiscoveryStore } from '../mcp-tool-discovery';

const { discoverMcpTools } = vi.hoisted(() => ({
  discoverMcpTools: vi.fn(),
}));

vi.mock('../../lib/tauri', () => ({
  discoverMcpTools,
}));

describe('useMcpToolDiscoveryStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    discoverMcpTools.mockReset();
  });

  it('derives ready, auth, stale, and unavailable server health from discovery results', async () => {
    discoverMcpTools
      .mockResolvedValueOnce([
        { server: 'filesystem', type: 'stdio', status: 'ok', tools: [{ name: 'filesystem.ReadFile', inputSchema: {} }] },
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
  });
});
