import { StreamableHTTPError } from '@modelcontextprotocol/sdk/client/streamableHttp';
import { describe, expect, it, vi } from 'vitest';
import { createMcpClientOps, type SdkClient } from '../mcpClientOps';
import type { ToolCallResult } from '../mcpTransport';

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
  return createMcpClientOps({
    ensureConnectedSession: async () => clientProvider(),
    reconnectSession,
    setError: vi.fn(),
  });
}

describe('createMcpClientOps gateway routing', () => {
  it('builds a tool-to-domain map from GatewayIndex and routes domain tools via GatewayExecute', async () => {
    const client = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in args)) {
        return toolText({
          domains: [
            { name: 'maenifold.system' },
            { name: 'maenifold.workflow' },
            { name: 'maenifold.memory' },
          ],
        });
      }

      if (name === 'GatewayIndex' && args.domain === 'maenifold.system') {
        return toolText({ commands: [{ name: 'GetConfig' }] });
      }

      if (name === 'GatewayIndex' && args.domain === 'maenifold.workflow') {
        return toolText({ commands: [{ name: 'Pipeline' }] });
      }

      if (name === 'GatewayIndex' && args.domain === 'maenifold.memory') {
        return toolText({ commands: [{ name: 'ListMemories' }] });
      }

      if (name === 'GatewayExecute') {
        return toolText(args);
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool('GetConfig')).resolves.toEqual({
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    await expect(ops.callTool('Pipeline', { workflowId: 'wf-1' })).resolves.toEqual({
      domain: 'maenifold.workflow',
      command: 'Pipeline',
      parameters: { workflowId: 'wf-1' },
    });
    await expect(ops.callTool('ListMemories', { path: 'memory://projects' })).resolves.toEqual({
      domain: 'maenifold.memory',
      command: 'ListMemories',
      parameters: { path: 'memory://projects' },
    });

    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayIndex')).toEqual([
      [{ name: 'GatewayIndex', arguments: {} }],
      [{ name: 'GatewayIndex', arguments: { domain: 'maenifold.system' } }],
      [{ name: 'GatewayIndex', arguments: { domain: 'maenifold.workflow' } }],
      [{ name: 'GatewayIndex', arguments: { domain: 'maenifold.memory' } }],
    ]);
    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayExecute')).toEqual([
      [{ name: 'GatewayExecute', arguments: { domain: 'maenifold.system', command: 'GetConfig', parameters: {} } }],
      [{
        name: 'GatewayExecute',
        arguments: {
          domain: 'maenifold.workflow',
          command: 'Pipeline',
          parameters: { workflowId: 'wf-1' },
        },
      }],
      [{
        name: 'GatewayExecute',
        arguments: {
          domain: 'maenifold.memory',
          command: 'ListMemories',
          parameters: { path: 'memory://projects' },
        },
      }],
    ]);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'GetConfig')).toBe(false);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'Pipeline')).toBe(false);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'ListMemories')).toBe(false);
  });

  it('keeps passthrough tools as direct MCP calls', async () => {
    const client = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in args)) {
        return toolText({ domains: [{ name: 'maenifold.system' }] });
      }

      if (name === 'GatewayIndex' && args.domain === 'maenifold.system') {
        return toolText({ commands: [{ name: 'GetConfig' }] });
      }

      return toolText({ name, args });
    });

    const ops = createOps(() => client);

    await ops.callTool('GetConfig');
    await expect(ops.callTool('GetHelp', { toolName: 'ListMemories' })).resolves.toEqual({
      name: 'GetHelp',
      args: { toolName: 'ListMemories' },
    });
    await expect(ops.callToolRaw('GatewayIndex', { domain: 'maenifold.system' })).resolves.toEqual(
      toolText({
        commands: [{ name: 'GetConfig' }],
      }),
    );

    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayExecute')).toHaveLength(1);
    expect(client.callTool).toHaveBeenNthCalledWith(4, {
      name: 'GetHelp',
      arguments: { toolName: 'ListMemories' },
    });
    expect(client.callTool).toHaveBeenNthCalledWith(5, {
      name: 'GatewayIndex',
      arguments: { domain: 'maenifold.system' },
    });
  });

  it('requires positive direct-tool proof or a public allowlist seam before direct MCP fallback succeeds', async () => {
    const client = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex') {
        throw new Error('GatewayIndex unavailable');
      }

      if (name === 'status') {
        return toolText({ name, args });
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool('status', { verbose: true })).resolves.toEqual({
      name: 'status',
      args: { verbose: true },
    });
    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayIndex')).toHaveLength(1);
    expect(client.callTool).toHaveBeenLastCalledWith({
      name: 'status',
      arguments: { verbose: true },
    });
  });

  it('allows status direct fallback after a successful map build that does not include status', async () => {
    const client = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in args)) {
        return toolText({ domains: [] });
      }

      if (name === 'status') {
        return toolText({ name, args });
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool('status', { verbose: false })).resolves.toEqual({
      name: 'status',
      args: { verbose: false },
    });
    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayIndex')).toEqual([
      [{ name: 'GatewayIndex', arguments: {} }],
    ]);
    expect(client.callTool).toHaveBeenLastCalledWith({
      name: 'status',
      arguments: { verbose: false },
    });
  });

  it('does not make hidden gateway tools direct-callable when GatewayIndex is unavailable', async () => {
    const client = createMockClient(async ({ name }) => {
      if (name === 'GatewayIndex') {
        throw new Error('GatewayIndex unavailable');
      }

      if (name === 'GetConfig') {
        throw new Error('GetConfig should not be called directly');
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool('GetConfig')).rejects.toThrow(/GatewayIndex|gateway/i);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'GetConfig')).toBe(false);
  });

  it('does not direct-call non-hardcoded gateway tools like SearchMemories when GatewayIndex is unavailable', async () => {
    const client = createMockClient(async ({ name }) => {
      if (name === 'GatewayIndex') {
        throw new Error('GatewayIndex unavailable');
      }

      if (name === 'SearchMemories') {
        throw new Error('SearchMemories should not be called directly');
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool('SearchMemories', { query: 'gateway bypass' })).rejects.toThrow(/GatewayIndex|gateway/i);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'SearchMemories')).toBe(false);
  });

  it.each([
    {
      label: 'GatewayIndex throws',
      gatewayIndex: async () => {
        throw new Error('GatewayIndex unavailable');
      },
    },
    {
      label: 'GatewayIndex returns malformed JSON',
      gatewayIndex: async () => ({
        content: [{ type: 'text', text: '{not valid json' }],
      } satisfies ToolCallResult),
    },
  ])('does not silently direct-call domain tools when $label', async ({ gatewayIndex }) => {
    const client = createMockClient(async ({ name }) => {
      if (name === 'GatewayIndex') {
        return gatewayIndex();
      }

      if (name === 'GetConfig') {
        throw new Error('domain tool should not be called directly');
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool('GetConfig')).rejects.toThrow(/GatewayIndex|gateway/i);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'GetConfig')).toBe(false);
    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayIndex')).toHaveLength(1);
  });

  it.each([
    {
      label: 'plain text',
      errorText: 'GatewayIndex unavailable from MCP tool result',
    },
    {
      label: 'JSON-shaped text',
      errorText: '{"domains":[]}',
    },
  ])('treats GatewayIndex isError content as failure: $label', async ({ errorText }) => {
    const client = createMockClient(async ({ name }) => {
      if (name === 'GatewayIndex') {
        return toolErrorText(errorText);
      }

      if (name === 'GetConfig') {
        throw new Error('GetConfig should not be called directly after GatewayIndex isError');
      }

      if (name === 'GatewayExecute') {
        throw new Error('GatewayExecute should not be called after GatewayIndex isError');
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    let thrown: unknown;
    try {
      await ops.callTool('GetConfig');
    } catch (err: unknown) {
      thrown = err;
    }

    expect(thrown).toBeInstanceOf(Error);
    expect((thrown as Error).message).toContain('GatewayIndex returned MCP tool error');
    expect((thrown as Error).message).toContain(errorText);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'GetConfig')).toBe(false);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'GatewayExecute')).toBe(false);
    expect(client.callTool.mock.calls.filter(([request]) => request.name === 'GatewayIndex')).toHaveLength(1);
  });

  it.each([
    {
      label: 'root domains array is empty',
      toolName: 'GetConfig',
      args: {},
      rootIndex: { domains: [] },
      domainResponses: {},
    },
    {
      label: 'top-level empty object',
      toolName: 'GetConfig',
      args: {},
      rootIndex: {},
      domainResponses: {},
    },
    {
      label: 'missing domains field',
      toolName: 'Pipeline',
      args: { workflowId: 'wf-1' },
      rootIndex: { gateway: 'index' },
      domainResponses: {},
    },
    {
      label: 'domains is not an array',
      toolName: 'ListMemories',
      args: { path: 'memory://projects' },
      rootIndex: { domains: { name: 'maenifold.memory' } },
      domainResponses: {},
    },
    {
      label: 'domain detail missing commands',
      toolName: 'GetConfig',
      args: {},
      rootIndex: { domains: [{ name: 'maenifold.system' }] },
      domainResponses: {
        'maenifold.system': {},
      },
    },
    {
      label: 'domain detail commands array is empty',
      toolName: 'Pipeline',
      args: { workflowId: 'wf-empty' },
      rootIndex: { domains: [{ name: 'maenifold.workflow' }] },
      domainResponses: {
        'maenifold.workflow': { commands: [] },
      },
    },
    {
      label: 'domain detail commands is not an array',
      toolName: 'Pipeline',
      args: { workflowId: 'wf-2' },
      rootIndex: { domains: [{ name: 'maenifold.workflow' }] },
      domainResponses: {
        'maenifold.workflow': { commands: { name: 'Pipeline' } },
      },
    },
    {
      label: 'partial domain failure',
      toolName: 'ListMemories',
      args: { path: 'memory://teams' },
      rootIndex: {
        domains: [{ name: 'maenifold.system' }, { name: 'maenifold.memory' }],
      },
      domainResponses: {
        'maenifold.system': { commands: [{ name: 'GetConfig' }] },
        'maenifold.memory': {},
      },
    },
    {
      label: 'malformed domain array entry',
      toolName: 'SearchMemories',
      args: { query: 'memory' },
      rootIndex: { domains: [{}] },
      domainResponses: {},
    },
    {
      label: 'malformed command array entry',
      toolName: 'SearchMemories',
      args: { query: 'workflow' },
      rootIndex: { domains: [{ name: 'maenifold.memory' }] },
      domainResponses: {
        'maenifold.memory': { commands: [{}] },
      },
    },
    {
      label: 'known routed tool missing from otherwise successful map',
      toolName: 'ListMemories',
      args: { path: 'memory://projects' },
      rootIndex: { domains: [{ name: 'maenifold.system' }] },
      domainResponses: {
        'maenifold.system': { commands: [{ name: 'GetConfig' }] },
      },
    },
  ])('fails closed for schema-malformed GatewayIndex: $label', async ({
    toolName,
    args,
    rootIndex,
    domainResponses,
  }) => {
    const client = createMockClient(async ({ name, arguments: requestArgs = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in requestArgs)) {
        return toolText(rootIndex);
      }

      if (name === 'GatewayIndex' && typeof requestArgs.domain === 'string') {
        return toolText(domainResponses[requestArgs.domain as keyof typeof domainResponses] ?? {});
      }

      if (name === toolName) {
        throw new Error(`${toolName} should not be called directly`);
      }

      if (name === 'GatewayExecute') {
        return toolText(requestArgs);
      }

      throw new Error(`Unexpected direct call for ${name}`);
    });

    const ops = createOps(() => client);

    await expect(ops.callTool(toolName, args)).rejects.toThrow(/GatewayIndex|gateway|domain|command|directly/i);
    expect(client.callTool.mock.calls.some(([request]) => request.name === toolName)).toBe(false);
    expect(client.callTool.mock.calls.some(([request]) => request.name === 'GatewayExecute')).toBe(false);
  });

  it('does not direct-call a previously discovered SearchMemories command after rebuild failure', async () => {
    const firstClient = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in args)) {
        return toolText({ domains: [{ name: 'maenifold.memory' }] });
      }

      if (name === 'GatewayIndex' && args.domain === 'maenifold.memory') {
        return toolText({ commands: [{ name: 'SearchMemories' }] });
      }

      if (name === 'GatewayExecute') {
        return toolText(args);
      }

      throw new Error(`Unexpected call for ${name}`);
    });

    const secondClient = createMockClient(async ({ name }) => {
      if (name === 'GatewayExecute') {
        throw new StreamableHTTPError(404, 'MCP session expired');
      }

      throw new Error(`Unexpected call for ${name}`);
    });

    const thirdClient = createMockClient(async ({ name }) => {
      if (name === 'GatewayIndex') {
        throw new Error('GatewayIndex unavailable during rebuild');
      }

      if (name === 'SearchMemories') {
        throw new Error('SearchMemories should not be called directly after rebuild failure');
      }

      throw new Error(`Unexpected call for ${name}`);
    });

    let currentClient: SdkClient = firstClient;
    const reconnectSession = vi.fn(async () => {
      currentClient = thirdClient;
    });
    const ops = createOps(() => currentClient, reconnectSession);

    await expect(ops.callTool('SearchMemories', { query: 'first pass' })).resolves.toEqual({
      domain: 'maenifold.memory',
      command: 'SearchMemories',
      parameters: { query: 'first pass' },
    });

    currentClient = secondClient;

    await expect(ops.callTool('SearchMemories', { query: 'after rebuild failure' })).rejects.toThrow(/GatewayIndex|gateway/i);
    expect(reconnectSession).toHaveBeenCalledTimes(1);
    expect(thirdClient.callTool.mock.calls.some(([request]) => request.name === 'SearchMemories')).toBe(false);
  });

  it('reconnects and rebuilds the domain map when GatewayIndex hits a session-expired 404', async () => {
    const firstClient = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in args)) {
        throw new StreamableHTTPError(404, 'MCP session expired');
      }

      throw new Error(`Unexpected call for ${name}`);
    });

    const secondClient = createMockClient(async ({ name, arguments: args = {} }) => {
      if (name === 'GatewayIndex' && !('domain' in args)) {
        return toolText({ domains: [{ name: 'maenifold.system' }] });
      }

      if (name === 'GatewayIndex' && args.domain === 'maenifold.system') {
        return toolText({ commands: [{ name: 'GetConfig' }] });
      }

      if (name === 'GatewayExecute') {
        return toolText(args);
      }

      throw new Error(`Unexpected call for ${name}`);
    });

    let currentClient: SdkClient = firstClient;
    const reconnectSession = vi.fn(async () => {
      currentClient = secondClient;
    });

    const ops = createOps(() => currentClient, reconnectSession);

    await expect(ops.callTool('GetConfig')).resolves.toEqual({
      domain: 'maenifold.system',
      command: 'GetConfig',
      parameters: {},
    });
    expect(reconnectSession).toHaveBeenCalledTimes(1);
    expect(secondClient.callTool.mock.calls.filter(([request]) => request.name === 'GatewayIndex')).toEqual([
      [{ name: 'GatewayIndex', arguments: {} }],
      [{ name: 'GatewayIndex', arguments: { domain: 'maenifold.system' } }],
    ]);
    expect(secondClient.callTool.mock.calls.filter(([request]) => request.name === 'GatewayExecute')).toEqual([
      [{ name: 'GatewayExecute', arguments: { domain: 'maenifold.system', command: 'GetConfig', parameters: {} } }],
    ]);
  });
});
