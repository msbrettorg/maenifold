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

function gatewayIndexText(payload: unknown) {
  return {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  };
}

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
}

describe('useMcpToolDiscoveryStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mcpClientMock.connected.value = true;
    mcpClientMock.connect.mockReset();
    mcpClientMock.callTool.mockReset();
  });

  it('enumerates multiple GatewayIndex domains and populates per-domain commands', async () => {
    mcpClientMock.callTool.mockImplementation(async (name: string, args: Record<string, unknown> = {}) => {
      if (name !== 'GatewayIndex') throw new Error(`Unexpected tool ${name}`);
      if (!('domain' in args)) {
        return gatewayIndexText({
          domains: [
            { name: 'maenifold.memory', description: 'First-party', command_count: 1 },
            { name: 'github-mcp-server', description: 'Upstream', command_count: 1 },
          ],
        });
      }
      if (args.domain === 'maenifold.memory') {
        return gatewayIndexText({
          domain: 'maenifold.memory',
          commands: [{
            name: 'ListMemories',
            description: 'List memories',
            parameters_schema: { type: 'object', properties: {} },
          }],
        });
      }
      if (args.domain === 'github-mcp-server') {
        return gatewayIndexText({
          domain: 'github-mcp-server',
          commands: [{
            name: 'search_repositories',
            description: 'Search repositories',
            parameters_schema: { type: 'object', properties: { query: { type: 'string' } } },
          }],
        });
      }
      throw new Error(`Unexpected domain ${String(args.domain)}`);
    });

    const store = useMcpToolDiscoveryStore();
    const results = await store.refreshTools();

    expect(results).toEqual([
      {
        server: 'maenifold.memory',
        type: 'gateway',
        status: 'ok',
        error: null,
        tools: [{
          name: 'ListMemories',
          description: 'List memories',
          inputSchema: { type: 'object', properties: {} },
          appUi: null,
        }],
      },
      {
        server: 'github-mcp-server',
        type: 'gateway',
        status: 'ok',
        error: null,
        tools: [{
          name: 'search_repositories',
          description: 'Search repositories',
          inputSchema: { type: 'object', properties: { query: { type: 'string' } } },
          appUi: null,
        }],
      },
    ]);
    expect(mcpClientMock.callTool.mock.calls).toEqual([
      ['GatewayIndex', {}],
      ['GatewayIndex', { domain: 'maenifold.memory', includeSchemas: true }],
      ['GatewayIndex', { domain: 'github-mcp-server', includeSchemas: true }],
    ]);
  });

  it('keeps healthy buckets when one GatewayIndex domain fails and reuses stale cache for that domain', async () => {
    mcpClientMock.callTool.mockImplementation(async (name: string, args: Record<string, unknown> = {}) => {
      if (name !== 'GatewayIndex') throw new Error(`Unexpected tool ${name}`);
      if (!('domain' in args)) {
        return gatewayIndexText({ domains: [{ name: 'alpha' }, { name: 'beta' }] });
      }
      if (args.domain === 'alpha') {
        return gatewayIndexText({
          domain: 'alpha',
          commands: [{ name: 'alpha.old', description: 'Alpha old', parameters_schema: { type: 'object' } }],
        });
      }
      return gatewayIndexText({
        domain: 'beta',
        commands: [{ name: 'beta.old', description: 'Beta old', parameters_schema: { type: 'object' } }],
      });
    });

    const store = useMcpToolDiscoveryStore();
    await store.refreshTools();

    mcpClientMock.callTool.mockImplementation(async (name: string, args: Record<string, unknown> = {}) => {
      if (name !== 'GatewayIndex') throw new Error(`Unexpected tool ${name}`);
      if (!('domain' in args)) {
        return gatewayIndexText({ domains: [{ name: 'alpha' }, { name: 'beta' }] });
      }
      if (args.domain === 'alpha') {
        throw new Error('alpha unavailable');
      }
      return gatewayIndexText({
        domain: 'beta',
        commands: [{ name: 'beta.new', description: 'Beta new', parameters_schema: { type: 'object', required: ['id'] } }],
      });
    });

    const results = await store.refreshTools();

    expect(results).toEqual([
      {
        server: 'alpha',
        type: 'gateway',
        status: 'error',
        error: 'alpha unavailable',
        tools: [{
          name: 'alpha.old',
          description: 'Alpha old',
          inputSchema: { type: 'object' },
          appUi: null,
        }],
      },
      {
        server: 'beta',
        type: 'gateway',
        status: 'ok',
        error: null,
        tools: [{
          name: 'beta.new',
          description: 'Beta new',
          inputSchema: { type: 'object', required: ['id'] },
          appUi: null,
        }],
      },
    ]);
  });

  it('does not let an aborted refresh overwrite newer GatewayIndex results', async () => {
    const firstTopLevel = createDeferred<unknown>();
    mcpClientMock.callTool
      .mockImplementationOnce(async () => firstTopLevel.promise)
      .mockImplementationOnce(async () => gatewayIndexText({ domains: [{ name: 'beta' }] }))
      .mockImplementationOnce(async () => gatewayIndexText({
        domain: 'beta',
        commands: [{ name: 'beta.command', parameters_schema: { type: 'object' } }],
      }));

    const store = useMcpToolDiscoveryStore();
    const firstRefresh = store.refreshTools();
    const secondRefresh = store.refreshTools();
    await secondRefresh;

    firstTopLevel.resolve(gatewayIndexText({ domains: [{ name: 'alpha' }] }));
    await firstRefresh;

    expect(store.serverResults).toEqual([
      {
        server: 'beta',
        type: 'gateway',
        status: 'ok',
        error: null,
        tools: [{
          name: 'beta.command',
          description: null,
          inputSchema: { type: 'object' },
          appUi: null,
        }],
      },
    ]);
    expect(mcpClientMock.callTool).toHaveBeenCalledTimes(3);
  });
});
