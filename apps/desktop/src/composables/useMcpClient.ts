import { readonly, ref } from 'vue';
import { DEFAULT_MCP_BASE_URL } from '../lib/constants';
import { getApiBase } from '../lib/api-base';
import { getErrorMessage } from './mcpTransport';
import {
  createMcpClientOps,
  createNotificationRouter,
  createSdkMcpClient,
  toServerCapabilities,
  type McpServerCapabilities,
  type SdkClient,
  type SdkTransport,
} from './mcpClientOps';

function resolveMcpUrl(baseUrl?: string) {
  const resolvedBase = (baseUrl ?? getApiBase()) || DEFAULT_MCP_BASE_URL;
  return resolvedBase.endsWith('/mcp') ? resolvedBase : `${resolvedBase}/mcp`;
}

export function useMcpClient(baseUrl?: string) {
  const sessionId = ref<string | null>(null);
  const connected = ref(false);
  const reconnecting = ref(false);
  const error = ref<string | null>(null);
  const serverCapabilities = ref<McpServerCapabilities | null>(null);

  const mcpUrl = resolveMcpUrl(baseUrl);
  const notificationRouter = createNotificationRouter();
  const disconnectListeners = new Set<() => void>();
  let client: SdkClient | null = null;
  let transport: SdkTransport | null = null;
  let reconnectPromise: Promise<void> | null = null;
  let disconnectRequested = false;

  function setError(message: string | null): void {
    error.value = message;
  }

  function resetState() {
    const hadActiveSession = connected.value || sessionId.value !== null;
    sessionId.value = null;
    connected.value = false;
    serverCapabilities.value = null;
    if (hadActiveSession) {
      disconnectListeners.forEach((listener) => {
        listener();
      });
    }
  }

  function updateConnectedState(nextClient: SdkClient, nextTransport: SdkTransport): void {
    sessionId.value = nextTransport.sessionId ?? null;
    serverCapabilities.value = toServerCapabilities(nextClient);
    connected.value = true;
    reconnecting.value = false;
    error.value = null;
  }

  async function reconnectSession(): Promise<void> {
    if (reconnectPromise) {
      return reconnectPromise;
    }

    connected.value = false;
    reconnecting.value = true;
    disconnectRequested = false;
    reconnectPromise = (async () => {
      let timeoutHandle: ReturnType<typeof setTimeout>;
      try {
        const timeout = new Promise<never>((_, reject) => {
          timeoutHandle = setTimeout(() => reject(new Error('MCP reconnect timed out')), 30_000);
        });
        await Promise.race([connect({ isReconnect: true }), timeout]);
        if (disconnectRequested) {
          await disconnect();
        }
      } finally {
        clearTimeout(timeoutHandle!);
        reconnecting.value = false;
        reconnectPromise = null;
      }
    })();

    return reconnectPromise;
  }

  async function connect(options: { isReconnect?: boolean } = {}): Promise<void> {
    error.value = null;
    resetState();

    try {
      await client?.close().catch(() => undefined);
      const created = createSdkMcpClient({
        mcpUrl,
        notificationRouter,
        onError: setError,
        onClose: resetState,
      });
      client = created.client;
      transport = created.transport;
      await client.connect(transport);
      updateConnectedState(client, transport);
    } catch (err: unknown) {
      const message = getErrorMessage(err);
      resetState();
      client = null;
      transport = null;
      if (!options.isReconnect) {
        reconnecting.value = false;
      }
      error.value = message;

      if (err instanceof Error) {
        throw err;
      }

      throw new Error(message);
    }
  }

  async function disconnect(): Promise<void> {
    disconnectRequested = true;
    const currentClient = client;
    const currentTransport = transport;

    try {
      await currentTransport?.terminateSession().catch(() => undefined);
      await currentClient?.close().catch(() => undefined);
    } finally {
      client = null;
      transport = null;
      reconnecting.value = false;
      reconnectPromise = null;
      resetState();
      error.value = null;
    }
  }

  async function ensureConnectedSession(): Promise<SdkClient> {
    if (connected.value && client) return client;
    if (reconnecting.value && reconnectPromise) {
      await reconnectPromise;
    }
    if (!connected.value || !client) {
      error.value = 'MCP client is not connected';
      throw new Error(error.value);
    }
    return client;
  }

  const ops = createMcpClientOps({
    ensureConnectedSession,
    reconnectSession,
    setError,
  });

  function supportsResourceSubscribe(): boolean {
    return serverCapabilities.value?.resources?.subscribe === true;
  }

  function supportsPrompts(): boolean {
    return serverCapabilities.value?.prompts != null;
  }

  function onDisconnect(handler: () => void): () => void {
    disconnectListeners.add(handler);
    return () => {
      disconnectListeners.delete(handler);
    };
  }

  return {
    sessionId: readonly(sessionId),
    connected: readonly(connected),
    reconnecting: readonly(reconnecting),
    error: readonly(error),
    serverCapabilities: readonly(serverCapabilities),
    connect,
    disconnect,
    reconnectSession,
    callTool: ops.callTool,
    callToolRaw: ops.callToolRaw,
    listTools: ops.listTools,
    readResource: ops.readResource,
    subscribeResource: ops.subscribeResource,
    unsubscribeResource: ops.unsubscribeResource,
    listResources: ops.listResources,
    listResourceTemplates: ops.listResourceTemplates,
    listPrompts: ops.listPrompts,
    getPrompt: ops.getPrompt,
    supportsResourceSubscribe,
    supportsPrompts,
    onNotification: notificationRouter.onNotification,
    onDisconnect,
  };
}
