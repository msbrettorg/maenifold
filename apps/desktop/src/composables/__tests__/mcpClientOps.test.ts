import { StreamableHTTPError } from '@modelcontextprotocol/sdk/client/streamableHttp';
import { describe, expect, it, vi } from 'vitest';
import { createMcpClientOps, type SdkClient } from '../mcpClientOps';
import type { ToolCallResult } from '../mcpTransport';
import { ToolEnvelopeError } from '../pipeline-mcp-transport';

function toolText(payload: unknown): ToolCallResult {
  return {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  };
}

function toolErrorText(text: string): ToolCallResult {
  return {
    content: [{ type: 'text', text }],
    isError: true,
  };
}

function createMockClient(
  callToolImpl: (request: { name: string; arguments?: Record<string, unknown> }) => Promise<ToolCallResult>,
): SdkClient & { callTool: ReturnType<typeof vi.fn> } {
  return {
    callTool: vi.fn(callToolImpl),
  } as unknown as SdkClient & { callTool: ReturnType<typeof vi.fn> };
}

function createOps(
  clientProvider: () => SdkClient,
  reconnectSession: () => Promise<void> = async () => undefined,
) {
  const setError = vi.fn();
  return {
    setError,
    ops: createMcpClientOps({
      ensureConnectedSession: async () => clientProvider(),
      reconnectSession,
      setError,
    }),
  };
}

describe('createMcpClientOps direct daemon routing', () => {
  it('passes routed and canonical tool names straight through to the daemon', async () => {
    const client = createMockClient(async ({ name, arguments: args = {} }) => toolText({ name, args }));
    const { ops } = createOps(() => client);

    await expect(ops.callTool('SearchMemories', { query: 'memory' })).resolves.toEqual({
      name: 'SearchMemories',
      args: { query: 'memory' },
    });
    await expect(ops.callTool('GatewayIndex', { domain: 'maenifold.memory' })).resolves.toEqual({
      name: 'GatewayIndex',
      args: { domain: 'maenifold.memory' },
    });
    await expect(ops.callTool('GetHelp', { toolName: 'SearchMemories' })).resolves.toEqual({
      name: 'GetHelp',
      args: { toolName: 'SearchMemories' },
    });

    expect(client.callTool.mock.calls).toEqual([
      [{ name: 'SearchMemories', arguments: { query: 'memory' } }],
      [{ name: 'GatewayIndex', arguments: { domain: 'maenifold.memory' } }],
      [{ name: 'GetHelp', arguments: { toolName: 'SearchMemories' } }],
    ]);
  });

  it('lets unknown tool errors come back from the daemon instead of rejecting in the frontend', async () => {
    const client = createMockClient(async ({ name }) => {
      throw new Error(`-32601 Method not found: ${name}`);
    });
    const { ops, setError } = createOps(() => client);

    await expect(ops.callTool('DiscoverMcpPrompts')).rejects.toThrow('-32601 Method not found: DiscoverMcpPrompts');
    expect(client.callTool).toHaveBeenCalledWith({
      name: 'DiscoverMcpPrompts',
      arguments: {},
    });
    expect(setError).toHaveBeenLastCalledWith('-32601 Method not found: DiscoverMcpPrompts');
  });

  it('reconnects and retries the same direct call after a session-expired 404', async () => {
    const firstClient = createMockClient(async () => {
      throw new StreamableHTTPError(404, 'MCP session expired');
    });
    const secondClient = createMockClient(async ({ name, arguments: args = {} }) => toolText({ name, args }));

    let currentClient: SdkClient = firstClient;
    const reconnectSession = vi.fn(async () => {
      currentClient = secondClient;
    });
    const { ops } = createOps(() => currentClient, reconnectSession);

    await expect(ops.callTool('SearchMemories', { query: 'after reconnect' })).resolves.toEqual({
      name: 'SearchMemories',
      args: { query: 'after reconnect' },
    });
    expect(reconnectSession).toHaveBeenCalledTimes(1);
    expect(secondClient.callTool).toHaveBeenCalledWith({
      name: 'SearchMemories',
      arguments: { query: 'after reconnect' },
    });
  });

  it('throws ToolEnvelopeError for structured tool errors returned by the daemon', async () => {
    const client = createMockClient(async () => toolErrorText(JSON.stringify({
      success: false,
      data: null,
      error: { code: 'NOT_FOUND', message: 'Memory file not found: ConceptX' },
    })));
    const { ops, setError } = createOps(() => client);

    await expect(ops.callTool('ReadMemory', { identifier: 'memory://ConceptX' })).rejects.toMatchObject({
      name: 'ToolEnvelopeError',
      code: 'NOT_FOUND',
      message: 'Memory file not found: ConceptX',
    } satisfies Partial<ToolEnvelopeError>);
    expect(setError).toHaveBeenLastCalledWith('Memory file not found: ConceptX');
  });
});
