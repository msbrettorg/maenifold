// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { startMcpLoopbackServer } from '../../__tests__/helpers/mcp-loopback';
import { getSharedMcpClient } from '../useSharedMcpClient';
import { McpCapabilityUnsupportedError, useMcpResources } from '../useMcpResources';

let currentServer: Awaited<ReturnType<typeof startMcpLoopbackServer>> | null = null;
let currentBaseUrl: string | null = null;

async function startResourceServer(resources = [{ uri: 'memory://resource/default', mimeType: 'text/plain', text: 'hello' }]) {
  currentServer = await startMcpLoopbackServer({ resources });
  currentBaseUrl = currentServer.baseUrl;
  return { server: currentServer, baseUrl: currentBaseUrl };
}

function bodyMethod(body: unknown): string | undefined {
  return typeof body === 'object' && body !== null && 'method' in body
    ? String((body as { method?: unknown }).method)
    : undefined;
}

function requestsForMethod(method: string) {
  return currentServer?.requests.filter((request) => bodyMethod(request.body) === method) ?? [];
}

describe('useMcpResources', () => {
  afterEach(async () => {
    if (currentBaseUrl) {
      await getSharedMcpClient(currentBaseUrl).disconnect().catch(() => undefined);
    }
    await currentServer?.close().catch(() => undefined);
    currentBaseUrl = null;
    currentServer = null;
  });

  it('readResource returns parsed resource content from a real MCP SDK server', async () => {
    const uri = 'memory://resource/a';
    const { baseUrl } = await startResourceServer([{ uri, mimeType: 'text/plain', text: 'hello' }]);

    const resources = useMcpResources(baseUrl);

    await expect(resources.readResource(uri)).resolves.toEqual({
      uri,
      mimeType: 'text/plain',
      text: 'hello',
    });
    expect(requestsForMethod('resources/read')).toHaveLength(1);
  });

  it('subscribe dispatches notifications to the registered handler', async () => {
    const uri = 'memory://resource/b';
    const handler = vi.fn();
    const { baseUrl, server } = await startResourceServer([{ uri, mimeType: 'text/plain', text: 'hello' }]);

    const resources = useMcpResources(baseUrl);
    await resources.subscribeResource(uri, handler);
    server.notifyOnNextList(uri);
    await resources.listResources();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith(uri);
  });

  it('fanouts notifications to multiple subscribers for the same uri', async () => {
    const uri = 'memory://resource/c';
    const first = vi.fn();
    const second = vi.fn();
    const { baseUrl, server } = await startResourceServer([{ uri, mimeType: 'text/plain', text: 'hello' }]);

    const resources = useMcpResources(baseUrl);
    await resources.subscribeResource(uri, first);
    await resources.subscribeResource(uri, second);
    server.notifyOnNextList(uri);
    await resources.listResources();

    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
    expect(requestsForMethod('resources/subscribe')).toHaveLength(1);
  });

  it('unsubscribeResource removes local handlers and stops notification callbacks', async () => {
    const uri = 'memory://resource/d';
    const handler = vi.fn();
    const { baseUrl, server } = await startResourceServer([{ uri, mimeType: 'text/plain', text: 'hello' }]);

    const resources = useMcpResources(baseUrl);
    await resources.subscribeResource(uri, handler);
    await resources.unsubscribeResource(uri);
    server.notifyOnNextList(uri);
    await resources.listResources();

    expect(handler).not.toHaveBeenCalled();
    expect(requestsForMethod('resources/unsubscribe')).toHaveLength(1);
  });

  it('disconnect clears local subscription registry without sending unsubscribe', async () => {
    const uri = 'memory://resource/e';
    const handler = vi.fn();
    const { baseUrl, server } = await startResourceServer([{ uri, mimeType: 'text/plain', text: 'hello' }]);

    const resources = useMcpResources(baseUrl);
    const mcp = getSharedMcpClient(baseUrl);
    await resources.subscribeResource(uri, handler);
    await mcp.disconnect();
    server.notifyOnNextList(uri);
    await resources.listResources();

    expect(handler).not.toHaveBeenCalled();
    expect(requestsForMethod('resources/unsubscribe')).toHaveLength(0);
  });

  it('throws McpCapabilityUnsupportedError when resources.subscribe capability is not advertised', async () => {
    currentServer = await startMcpLoopbackServer({ resourcesSubscribe: false });
    currentBaseUrl = currentServer.baseUrl;

    const resources = useMcpResources(currentBaseUrl);

    await expect(resources.subscribeResource('memory://resource/f', vi.fn())).rejects.toBeInstanceOf(
      McpCapabilityUnsupportedError,
    );
    expect(requestsForMethod('resources/subscribe')).toHaveLength(0);
  });
});
