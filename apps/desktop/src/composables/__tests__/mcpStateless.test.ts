// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { startMcpLoopbackServer } from '../../__tests__/helpers/mcp-loopback';
import { useMcpClient } from '../useMcpClient';

let currentClient: ReturnType<typeof useMcpClient> | null = null;
let currentServer: Awaited<ReturnType<typeof startMcpLoopbackServer>> | null = null;

async function startClient(stateless = true) {
  currentServer = await startMcpLoopbackServer({ stateless });
  currentClient = useMcpClient(currentServer.baseUrl);
  return { client: currentClient, server: currentServer };
}

function bodyMethod(body: unknown): string | undefined {
  return typeof body === 'object' && body !== null && 'method' in body
    ? String((body as { method?: unknown }).method)
    : undefined;
}

function callToolName(body: unknown): string | undefined {
  if (typeof body !== 'object' || body === null || !('params' in body)) {
    return undefined;
  }
  const params = (body as { params?: unknown }).params;
  if (typeof params !== 'object' || params === null || !('name' in params)) {
    return undefined;
  }
  return String((params as { name?: unknown }).name);
}

function requestsForMethod(method: string) {
  return currentServer?.requests.filter((request) => bodyMethod(request.body) === method) ?? [];
}

describe('useMcpClient stateless MCP support', () => {
  afterEach(async () => {
    await currentClient?.disconnect().catch(() => undefined);
    await currentServer?.close().catch(() => undefined);
    currentClient = null;
    currentServer = null;
  });

  it('connect() succeeds against a real stateless MCP SDK server', async () => {
    const { client, server } = await startClient();

    await expect(client.connect()).resolves.toBeUndefined();

    expect(server.requests.map((request) => bodyMethod(request.body))).toEqual([
      'initialize',
      'notifications/initialized',
    ]);
  });

  it('keeps connected=true and sessionId=null after a stateless connect', async () => {
    const { client } = await startClient();

    await client.connect();

    expect(client.connected.value).toBe(true);
    expect(client.sessionId.value).toBeNull();
    expect(client.error.value).toBeNull();
  });

  it('builds initialize requests with Mcp-Protocol-Version and without Mcp-Session-Id', async () => {
    const { client, server } = await startClient();

    await client.connect();

    const initializeRequest = server.requests.find((request) => bodyMethod(request.body) === 'initialize');
    expect(initializeRequest?.headers).toMatchObject({
      accept: 'application/json, text/event-stream',
      'content-type': 'application/json',
      'mcp-protocol-version': '2025-03-26',
    });
    expect(initializeRequest?.headers['mcp-session-id']).toBeUndefined();
  });

  it('listTools() works after a stateless connect even without a session id', async () => {
    const { client, server } = await startClient();
    await client.connect();

    await expect(client.listTools()).resolves.toEqual(
      expect.arrayContaining([expect.objectContaining({ name: 'Pipeline' })]),
    );

    const toolsListRequest = server.requests.find((request) => bodyMethod(request.body) === 'tools/list');
    expect(toolsListRequest?.headers['mcp-session-id']).toBeUndefined();
  });

  it('callTool() routes through GatewayExecute with the real SDK transport in stateless mode', async () => {
    const { client, server } = await startClient();
    await client.connect();

    await expect(client.callTool('Pipeline', { list: true })).resolves.toEqual({
      success: true,
      data: { pipelines: [] },
    });

    const toolNames = server.requests
      .filter((request) => bodyMethod(request.body) === 'tools/call')
      .map((request) => callToolName(request.body));
    expect(toolNames).toEqual([
      'GatewayIndex',
      'GatewayIndex',
      'GatewayExecute',
    ]);
    expect(server.requests.every((request) => request.headers['mcp-session-id'] === undefined)).toBe(true);
  });

  it('reconnects and retries after a 404 in stateless mode', async () => {
    currentServer = await startMcpLoopbackServer({
      stateless: true,
      failOnce: (request) => (
        bodyMethod(request.body) === 'tools/call'
        && callToolName(request.body) === 'GatewayExecute'
      ),
    });
    currentClient = useMcpClient(currentServer.baseUrl);

    await currentClient.connect();

    await expect(currentClient.callTool('Pipeline', { list: true })).resolves.toEqual({
      success: true,
      data: { pipelines: [] },
    });
    expect(requestsForMethod('initialize')).toHaveLength(2);
    expect(
      currentServer.requests
        .filter((request) => bodyMethod(request.body) === 'tools/call')
        .map((request) => callToolName(request.body)),
    ).toEqual([
      'GatewayIndex',
      'GatewayIndex',
      'GatewayExecute',
      'GatewayIndex',
      'GatewayIndex',
      'GatewayExecute',
    ]);
  });

  it('disconnect() skips DELETE when there is no session id', async () => {
    const { client, server } = await startClient();
    await client.connect();
    await client.disconnect();

    expect(server.requests.some((request) => request.method === 'DELETE')).toBe(false);
    expect(client.connected.value).toBe(false);
    expect(client.sessionId.value).toBeNull();
  });

  it('preserves stateful behavior when the server returns a session id', async () => {
    const { client, server } = await startClient(false);

    await client.connect();
    await expect(client.listTools()).resolves.toEqual(
      expect.arrayContaining([expect.objectContaining({ name: 'Pipeline' })]),
    );

    expect(client.connected.value).toBe(true);
    expect(client.sessionId.value).toBe('session-1');
    expect(server.requests.find((request) => bodyMethod(request.body) === 'initialize')?.headers['mcp-session-id'])
      .toBeUndefined();
    expect(server.requests.find((request) => bodyMethod(request.body) === 'notifications/initialized')?.headers['mcp-session-id'])
      .toBe('session-1');
    expect(server.requests.find((request) => bodyMethod(request.body) === 'tools/list')?.headers['mcp-session-id'])
      .toBe('session-1');
  });
});
