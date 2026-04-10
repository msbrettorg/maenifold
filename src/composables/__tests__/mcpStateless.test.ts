// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMcpClient } from '../useMcpClient';

const { fetchMock } = vi.hoisted(() => ({
  fetchMock: vi.fn(),
}));

vi.mock('../../lib/http', () => ({
  getTauriFetch: vi.fn(async () => fetchMock),
}));

const BASE_URL = 'http://127.0.0.1:8080';
const MCP_URL = `${BASE_URL}/mcp`;

function jsonRpcResponse(id: number, result: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify({ jsonrpc: '2.0', id, result }), {
    status: 200,
    headers: {
      'content-type': 'application/json',
      ...headers,
    },
  });
}

function notificationResponse(headers: Record<string, string> = {}): Response {
  return new Response(null, {
    status: 202,
    headers,
  });
}

function toolCallResponse(id: number, payload: unknown): Response {
  return jsonRpcResponse(id, {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  });
}

function listToolsResponse(id: number, toolNames: string[]): Response {
  return jsonRpcResponse(id, {
    tools: toolNames.map((name) => ({ name })),
  });
}

function getHeadersAt(callIndex: number): Record<string, string> {
  return (fetchMock.mock.calls[callIndex]?.[1] as RequestInit | undefined)?.headers as Record<string, string>;
}

describe('useMcpClient stateless MCP support', () => {
  beforeEach(() => {
    fetchMock.mockReset();
  });

  it('connect() succeeds when initialize returns no Mcp-Session-Id header', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse());

    const client = useMcpClient(BASE_URL);

    await expect(client.connect()).resolves.toBeUndefined();
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      MCP_URL,
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('keeps connected=true and sessionId=null after a stateless connect', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse());

    const client = useMcpClient(BASE_URL);

    await client.connect();

    expect(client.connected.value).toBe(true);
    expect(client.sessionId.value).toBeNull();
    expect(client.error.value).toBeNull();
  });

  it('builds requests with Mcp-Protocol-Version when no session id exists', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse());

    const client = useMcpClient(BASE_URL);

    await client.connect().catch(() => undefined);

    expect(getHeadersAt(0)).toMatchObject({
      Accept: 'application/json, text/event-stream',
      'Content-Type': 'application/json',
      'Mcp-Protocol-Version': '2025-03-26',
    });
    expect(getHeadersAt(0)['Mcp-Session-Id']).toBeUndefined();
  });

  it('listTools() works after a stateless connect even without a session id', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(listToolsResponse(2, ['Pipeline']));

    const client = useMcpClient(BASE_URL);
    await client.connect();

    await expect(client.listTools()).resolves.toEqual([{ name: 'Pipeline' }]);
    expect(getHeadersAt(2)['Mcp-Session-Id']).toBeUndefined();
  });

  it('callTool() works in stateless mode', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(toolCallResponse(2, { success: true, data: { pipelines: [] } }));

    const client = useMcpClient(BASE_URL);
    await client.connect();

    await expect(client.callTool('Pipeline', { list: true })).resolves.toEqual({
      success: true,
      data: { pipelines: [] },
    });
    expect(getHeadersAt(2)['Mcp-Session-Id']).toBeUndefined();
  });

  it('reconnects and retries after a 404 in stateless mode', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(new Response(null, { status: 404 }))
      .mockResolvedValueOnce(jsonRpcResponse(3, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(toolCallResponse(2, { success: true, data: { pipelines: [] } }));

    const client = useMcpClient(BASE_URL);
    await client.connect();

    await expect(client.callTool('Pipeline', { list: true })).resolves.toEqual({
      success: true,
      data: { pipelines: [] },
    });
    expect(fetchMock).toHaveBeenCalledTimes(6);
    expect(getHeadersAt(5)['Mcp-Session-Id']).toBeUndefined();
  });

  it('disconnect() skips DELETE when there is no session id', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonRpcResponse(1, { protocolVersion: '2025-03-26' }))
      .mockResolvedValueOnce(notificationResponse());

    const client = useMcpClient(BASE_URL);
    await client.connect();
    await client.disconnect();

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(client.connected.value).toBe(false);
    expect(client.sessionId.value).toBeNull();
  });

  it('preserves stateful behavior when the server returns a session id', async () => {
    fetchMock
      .mockResolvedValueOnce(
        jsonRpcResponse(1, { protocolVersion: '2025-03-26' }, { 'Mcp-Session-Id': 'session-123' }),
      )
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(listToolsResponse(2, ['Pipeline']));

    const client = useMcpClient(BASE_URL);

    await client.connect();
    await expect(client.listTools()).resolves.toEqual([{ name: 'Pipeline' }]);

    expect(client.connected.value).toBe(true);
    expect(client.sessionId.value).toBe('session-123');
    expect(getHeadersAt(1)['Mcp-Session-Id']).toBe('session-123');
    expect(getHeadersAt(2)['Mcp-Session-Id']).toBe('session-123');
  });
});
