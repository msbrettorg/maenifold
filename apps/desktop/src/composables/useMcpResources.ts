import type {
  ResourceContent,
  ResourceDescriptor,
  ResourceTemplateDescriptor,
} from './mcpTransport';
import { getSharedMcpClient } from './usePipelineMcp';

export class McpCapabilityUnsupportedError extends Error {
  capability: string;

  constructor(capability: string) {
    super(`MCP server does not support capability: ${capability}`);
    this.name = 'McpCapabilityUnsupportedError';
    this.capability = capability;
  }
}

type ResourceUpdateHandler = (uri: string) => void;

interface ResourceNotificationParams {
  uri?: unknown;
}

interface McpResourcesClient {
  readResource: (uri: string) => Promise<ResourceContent>;
  subscribeResource: (uri: string, handler: ResourceUpdateHandler) => Promise<() => void>;
  unsubscribeResource: (uri: string) => Promise<void>;
  listResources: () => Promise<ResourceDescriptor[]>;
  listResourceTemplates: () => Promise<ResourceTemplateDescriptor[]>;
}

const sharedMcpResources = new Map<string, McpResourcesClient>();

/**
 * MCP resources composable layered on top of the shared MCP transport/session.
 *
 * Subscription lifecycle contract:
 * - Local handlers are cleared when the MCP session disconnects.
 * - No server-side unsubscribe is attempted during disconnect teardown.
 * - After reconnect, consumers must subscribe again.
 */
export function useMcpResources(baseUrl?: string): McpResourcesClient {
  const key = baseUrl ?? '__default__';
  const existing = sharedMcpResources.get(key);
  if (existing) {
    return existing;
  }

  const mcp = getSharedMcpClient(baseUrl);
  const handlersByUri = new Map<string, Set<ResourceUpdateHandler>>();
  const serverSubscribedUris = new Set<string>();

  function parseUpdatedUri(params: unknown): string | null {
    if (!params || typeof params !== 'object') {
      return null;
    }
    const { uri } = params as ResourceNotificationParams;
    return typeof uri === 'string' ? uri : null;
  }

  mcp.onNotification((notification) => {
    if (notification.method !== 'notifications/resources/updated') {
      return;
    }

    const uri = parseUpdatedUri(notification.params);
    if (!uri) {
      return;
    }

    const handlers = handlersByUri.get(uri);
    if (!handlers || handlers.size === 0) {
      console.debug('[useMcpResources] Dropping resource update for unsubscribed uri', uri); // audit-ok: no-console
      return;
    }

    handlers.forEach((handler) => {
      handler(uri);
    });
  });

  mcp.onDisconnect(() => {
    handlersByUri.clear();
    serverSubscribedUris.clear();
  });

  async function ensureConnected(): Promise<void> {
    if (mcp.connected.value) {
      return;
    }
    await mcp.connect();
  }

  async function readResource(uri: string): Promise<ResourceContent> {
    await ensureConnected();
    return mcp.readResource(uri);
  }

  async function listResources(): Promise<ResourceDescriptor[]> {
    await ensureConnected();
    return mcp.listResources();
  }

  async function listResourceTemplates(): Promise<ResourceTemplateDescriptor[]> {
    await ensureConnected();
    return mcp.listResourceTemplates();
  }

  async function unsubscribeSpecificHandler(uri: string, handler: ResourceUpdateHandler): Promise<void> {
    const handlers = handlersByUri.get(uri);
    if (!handlers) {
      return;
    }

    handlers.delete(handler);
    if (handlers.size > 0) {
      return;
    }

    handlersByUri.delete(uri);
    if (!serverSubscribedUris.delete(uri)) {
      return;
    }

    await ensureConnected();
    await mcp.unsubscribeResource(uri);
  }

  async function subscribeResource(uri: string, handler: ResourceUpdateHandler): Promise<() => void> {
    await ensureConnected();

    if (!mcp.supportsResourceSubscribe()) {
      throw new McpCapabilityUnsupportedError('resources.subscribe');
    }

    let handlers = handlersByUri.get(uri);
    if (!handlers) {
      handlers = new Set<ResourceUpdateHandler>();
      handlersByUri.set(uri, handlers);
    }

    const sizeBeforeAdd = handlers.size;
    handlers.add(handler);
    const firstLocalSubscriber = sizeBeforeAdd === 0;

    if (firstLocalSubscriber && !serverSubscribedUris.has(uri)) {
      try {
        await mcp.subscribeResource(uri);
        serverSubscribedUris.add(uri);
      } catch (error) {
        handlers.delete(handler);
        if (handlers.size === 0) {
          handlersByUri.delete(uri);
        }
        throw error;
      }
    }

    return () => {
      void unsubscribeSpecificHandler(uri, handler);
    };
  }

  async function unsubscribeResource(uri: string): Promise<void> {
    handlersByUri.delete(uri);
    if (!serverSubscribedUris.delete(uri)) {
      return;
    }

    await ensureConnected();
    await mcp.unsubscribeResource(uri);
  }

  const client: McpResourcesClient = {
    readResource,
    subscribeResource,
    unsubscribeResource,
    listResources,
    listResourceTemplates,
  };

  sharedMcpResources.set(key, client);
  return client;
}
