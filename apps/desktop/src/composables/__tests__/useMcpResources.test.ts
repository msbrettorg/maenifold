// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getSharedMcpClient } from '../usePipelineMcp';
import { McpCapabilityUnsupportedError, useMcpResources } from '../useMcpResources';

const { fetchMock } = vi.hoisted(() => ({
  fetchMock: vi.fn(),
}));

vi.mock('../../lib/http', () => ({
  getHttpFetch: vi.fn(() => fetchMock),
}));

function jsonRpcResponse(id: number, result: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify({ jsonrpc: '2.0', id, result }), {
    status: 200,
    headers: {
      'content-type': 'application/json',
      ...headers,
    },
  });
}

function initializeResponse(id: number, subscribeCapability = true): Response {
  const capabilities = subscribeCapability
    ? { resources: { subscribe: true } }
    : {};
  return jsonRpcResponse(id, {
    protocolVersion: '2025-03-26',
    capabilities,
  });
}

function notificationResponse(): Response {
  return new Response(null, { status: 202 });
}

function sseResponse(body: string): Response {
  return new Response(body, {
    status: 200,
    headers: { 'content-type': 'text/event-stream' },
  });
}

function parseRequestMethod(callIndex: number): string {
  const init = fetchMock.mock.calls[callIndex]?.[1] as RequestInit | undefined;
  const body = init?.body;
  if (typeof body !== 'string') {
    return '';
  }
  return (JSON.parse(body) as { method?: string }).method ?? '';
}

function nextBaseUrl(index: number): string {
  return `http://127.0.0.1:${8500 + index}`;
}

describe('useMcpResources', () => {
  beforeEach(() => {
    fetchMock.mockReset();
  });

  it('readResource returns parsed resource content', async () => {
    const uri = 'memory://resource/a';
    const baseUrl = nextBaseUrl(1);
    fetchMock
      .mockResolvedValueOnce(initializeResponse(1))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(jsonRpcResponse(2, {
        contents: [{ uri, mimeType: 'text/plain', text: 'hello' }],
      }));

    const resources = useMcpResources(baseUrl);
    await expect(resources.readResource(uri)).resolves.toEqual({
      uri,
      mimeType: 'text/plain',
      text: 'hello',
    });
    expect(parseRequestMethod(2)).toBe('resources/read');
  });

  it('subscribe dispatches notifications to the registered handler', async () => {
    const uri = 'memory://resource/b';
    const baseUrl = nextBaseUrl(2);
    const handler = vi.fn();
    fetchMock
      .mockResolvedValueOnce(initializeResponse(1))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(jsonRpcResponse(2, {}))
      .mockResolvedValueOnce(sseResponse(
        `event: message
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"${uri}"}}

event: message
data: {"jsonrpc":"2.0","id":3,"result":{"resources":[]}}

`,
      ));

    const resources = useMcpResources(baseUrl);
    await resources.subscribeResource(uri, handler);
    await resources.listResources();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith(uri);
  });

  it('fanouts notifications to multiple subscribers for the same uri', async () => {
    const uri = 'memory://resource/c';
    const baseUrl = nextBaseUrl(3);
    const first = vi.fn();
    const second = vi.fn();
    fetchMock
      .mockResolvedValueOnce(initializeResponse(1))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(jsonRpcResponse(2, {}))
      .mockResolvedValueOnce(sseResponse(
        `event: message
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"${uri}"}}

event: message
data: {"jsonrpc":"2.0","id":3,"result":{"resources":[]}}

`,
      ));

    const resources = useMcpResources(baseUrl);
    await resources.subscribeResource(uri, first);
    await resources.subscribeResource(uri, second);
    await resources.listResources();

    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls.filter((_, i) => parseRequestMethod(i) === 'resources/subscribe')).toHaveLength(1);
  });

  it('unsubscribeResource removes local handlers and stops notification callbacks', async () => {
    const uri = 'memory://resource/d';
    const baseUrl = nextBaseUrl(4);
    const handler = vi.fn();
    fetchMock
      .mockResolvedValueOnce(initializeResponse(1))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(jsonRpcResponse(2, {}))
      .mockResolvedValueOnce(jsonRpcResponse(3, {}))
      .mockResolvedValueOnce(sseResponse(
        `event: message
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"${uri}"}}

event: message
data: {"jsonrpc":"2.0","id":4,"result":{"resources":[]}}

`,
      ));

    const resources = useMcpResources(baseUrl);
    await resources.subscribeResource(uri, handler);
    await resources.unsubscribeResource(uri);
    await resources.listResources();

    expect(handler).not.toHaveBeenCalled();
    expect(fetchMock.mock.calls.filter((_, i) => parseRequestMethod(i) === 'resources/unsubscribe')).toHaveLength(1);
  });

  it('disconnect clears local subscription registry without sending unsubscribe', async () => {
    const uri = 'memory://resource/e';
    const baseUrl = nextBaseUrl(5);
    const handler = vi.fn();
    fetchMock
      .mockResolvedValueOnce(
        jsonRpcResponse(
          1,
          { protocolVersion: '2025-03-26', capabilities: { resources: { subscribe: true } } },
          { 'Mcp-Session-Id': 'session-1' },
        ),
      )
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(jsonRpcResponse(2, {}))
      .mockResolvedValueOnce(new Response(null, { status: 200 }))
      .mockResolvedValueOnce(initializeResponse(3))
      .mockResolvedValueOnce(notificationResponse())
      .mockResolvedValueOnce(sseResponse(
        `event: message
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"${uri}"}}

event: message
data: {"jsonrpc":"2.0","id":4,"result":{"resources":[]}}

`,
      ));

    const resources = useMcpResources(baseUrl);
    const mcp = getSharedMcpClient(baseUrl);
    await resources.subscribeResource(uri, handler);
    await mcp.disconnect();
    await resources.listResources();

    expect(handler).not.toHaveBeenCalled();
    expect(fetchMock.mock.calls.filter((_, i) => parseRequestMethod(i) === 'resources/unsubscribe')).toHaveLength(0);
  });

  it('throws McpCapabilityUnsupportedError when resources.subscribe capability is not advertised', async () => {
    const baseUrl = nextBaseUrl(6);
    fetchMock
      .mockResolvedValueOnce(initializeResponse(1, false))
      .mockResolvedValueOnce(notificationResponse());

    const resources = useMcpResources(baseUrl);
    await expect(resources.subscribeResource('memory://resource/f', vi.fn())).rejects.toBeInstanceOf(
      McpCapabilityUnsupportedError,
    );
    expect(fetchMock.mock.calls.filter((_, i) => parseRequestMethod(i) === 'resources/subscribe')).toHaveLength(0);
  });
});
