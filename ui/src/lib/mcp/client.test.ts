import { afterEach, describe, expect, it, vi } from 'vitest';

import { McpHttpClient } from './client';

function jsonRpcResponse(result: unknown, init: ResponseInit = {}): Response {
  const headers = new Headers(init.headers);
  if (!headers.has('content-type')) {
    headers.set('content-type', 'application/json');
  }

  return new Response(JSON.stringify({ jsonrpc: '2.0', id: '1', result }), {
    ...init,
    headers,
  });
}

function sseJsonRpcResponse(result: unknown, init: ResponseInit = {}): Response {
  const headers = new Headers(init.headers);
  headers.set('content-type', 'text/event-stream');

  const body = [
    ': keep-alive',
    'id: 17',
    'retry: 5000',
    '',
    `data: ${JSON.stringify({ jsonrpc: '2.0', id: '1', result })}`,
    '',
  ].join('\n');

  return new Response(body, {
    ...init,
    headers,
  });
}

function emptyNotificationResponse(init: ResponseInit = {}): Response {
  return new Response('', {
    status: 202,
    ...init,
  });
}

function stubFetch(...responses: Response[]) {
  const fetchMock = vi.fn<typeof fetch>().mockImplementation(async () => {
    throw new Error('Unexpected fetch call');
  });

  for (const response of responses) {
    fetchMock.mockResolvedValueOnce(response);
  }

  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

function requestBody(fetchMock: ReturnType<typeof stubFetch>, callIndex: number) {
  const [, init] = fetchMock.mock.calls[callIndex] ?? [];
  return JSON.parse(String((init as RequestInit | undefined)?.body)) as {
    method: string;
    params?: unknown;
  };
}

function requestHeaders(fetchMock: ReturnType<typeof stubFetch>, callIndex: number) {
  const [, init] = fetchMock.mock.calls[callIndex] ?? [];
  return new Headers((init as RequestInit | undefined)?.headers);
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('McpHttpClient', () => {
  it('negotiates protocol version and accepts an empty initialized notification body', async () => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    const fetchMock = stubFetch(
      jsonRpcResponse({ protocolVersion: '2025-11-25' }),
      emptyNotificationResponse(),
      jsonRpcResponse({ contents: [{ text: '[]' }] })
    );

    await expect(client.readResource('schedule://jobs')).resolves.toEqual([]);

    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(requestBody(fetchMock, 0).method).toBe('initialize');
    expect(requestBody(fetchMock, 1).method).toBe('notifications/initialized');
    expect(requestBody(fetchMock, 2).method).toBe('resources/read');

    expect(requestHeaders(fetchMock, 0).has('MCP-Protocol-Version')).toBe(false);
    expect(requestHeaders(fetchMock, 1).get('MCP-Protocol-Version')).toBe('2025-11-25');
    expect(requestHeaders(fetchMock, 2).get('MCP-Protocol-Version')).toBe('2025-11-25');
  });

  it('sends only one initialize request for concurrent reads', async () => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    const fetchMock = stubFetch(
      jsonRpcResponse({ protocolVersion: '2025-11-25' }),
      emptyNotificationResponse(),
      jsonRpcResponse({ contents: [{ text: '[]' }] }),
      jsonRpcResponse({ contents: [{ text: '[{"id":"a"}]' }] })
    );

    await expect(
      Promise.all([client.readResource('schedule://jobs'), client.readResource('schedule://jobs')])
    ).resolves.toEqual([[], [{ id: 'a' }]]);

    const methods = fetchMock.mock.calls.map((_, index) => requestBody(fetchMock, index).method);
    expect(methods.filter((method) => method === 'initialize')).toHaveLength(1);
  });

  it('reuses mcp-session-id from the initialize response', async () => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    const fetchMock = stubFetch(
      jsonRpcResponse(
        { protocolVersion: '2025-11-25' },
        { headers: { 'mcp-session-id': 'session-123' } }
      ),
      emptyNotificationResponse(),
      jsonRpcResponse({ contents: [{ text: '[]' }] })
    );

    await client.readResource('schedule://jobs');

    expect(requestHeaders(fetchMock, 0).has('mcp-session-id')).toBe(false);
    expect(requestHeaders(fetchMock, 1).get('mcp-session-id')).toBe('session-123');
    expect(requestHeaders(fetchMock, 2).get('mcp-session-id')).toBe('session-123');
  });

  it.each([
    ['[]', []],
    ['[{"id":"a"}]', [{ id: 'a' }]],
  ])('double-parses resource text %s', async (resourceText, expected) => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    stubFetch(
      jsonRpcResponse({ protocolVersion: '2025-11-25' }),
      emptyNotificationResponse(),
      jsonRpcResponse({ contents: [{ text: resourceText }] })
    );

    await expect(client.readResource('schedule://jobs')).resolves.toEqual(expected);
  });

  it('throws McpHttpError when contents[0].text is missing', async () => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    stubFetch(
      jsonRpcResponse({ protocolVersion: '2025-11-25' }),
      emptyNotificationResponse(),
      jsonRpcResponse({ contents: [{}] })
    );

    await expect(client.readResource('schedule://jobs')).rejects.toMatchObject({
      name: 'McpHttpError',
      message: "MCP resource 'schedule://jobs' did not return contents[0].text",
    });
  });

  it('captures plain-text non-2xx bodies in McpHttpError', async () => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    stubFetch(
      jsonRpcResponse({ protocolVersion: '2025-11-25' }),
      emptyNotificationResponse(),
      new Response('upstream exploded', {
        status: 500,
        statusText: 'Internal Server Error',
        headers: { 'content-type': 'text/plain' },
      })
    );

    await expect(client.readResource('schedule://jobs')).rejects.toMatchObject({
      name: 'McpHttpError',
      status: 500,
      statusText: 'Internal Server Error',
      bodyText: 'upstream exploded',
    });
  });

  it('parses JSON-RPC payloads delivered as SSE', async () => {
    const client = new McpHttpClient('http://127.0.0.1:8080');
    stubFetch(
      jsonRpcResponse({ protocolVersion: '2025-11-25' }),
      emptyNotificationResponse(),
      sseJsonRpcResponse({ contents: [{ text: '[{"id":"a"}]' }] })
    );

    await expect(client.readResource('schedule://jobs')).resolves.toEqual([{ id: 'a' }]);
  });
});
