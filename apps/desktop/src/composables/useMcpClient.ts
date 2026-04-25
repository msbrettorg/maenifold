import { readonly, ref } from 'vue';
import { DEFAULT_MCP_BASE_URL, MCP_PROTOCOL_VERSION } from '../lib/constants';
import { getApiBase } from '../lib/api-base';
import { getHttpFetch } from '../lib/http';
import type {
  JsonRpcNotification,
  JsonRpcRequest,
  JsonRpcResponse,
  ResourceContent,
  ResourceDescriptor,
  ResourceTemplatesListResult,
  ResourceTemplateDescriptor,
  ResourcesListResult,
  ResourcesReadResult,
  ToolCallResult,
  ToolDefinition,
  ToolsListResult,
} from './mcpTransport';
import { getErrorMessage, parseJsonRpcResponse } from './mcpTransport';
import { extractAppUiMetadata } from '../stores/agent-capabilities-helpers';

function resolveMcpUrl(baseUrl?: string) {
  const resolvedBase = (baseUrl ?? getApiBase()) || DEFAULT_MCP_BASE_URL;
  return resolvedBase.endsWith('/mcp') ? resolvedBase : `${resolvedBase}/mcp`;
}

interface McpServerCapabilities {
  resources?: {
    subscribe?: boolean;
    listChanged?: boolean;
  };
  [key: string]: unknown;
}

interface InitializeResult {
  protocolVersion?: string;
  capabilities?: McpServerCapabilities;
  serverInfo?: {
    name?: string;
    version?: string;
  };
}

export function useMcpClient(baseUrl?: string) {
  const sessionId = ref<string | null>(null);
  const connected = ref(false);
  const reconnecting = ref(false);
  const error = ref<string | null>(null);
  const serverCapabilities = ref<McpServerCapabilities | null>(null);

  const mcpUrl = resolveMcpUrl(baseUrl);
  let nextId = 1;
  let reconnectPromise: Promise<void> | null = null;
  let disconnectRequested = false;
  const notificationListeners = new Set<(notification: JsonRpcNotification) => void>();
  const disconnectListeners = new Set<() => void>();

  function buildHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      Accept: 'application/json, text/event-stream',
      'Content-Type': 'application/json',
    };

    if (sessionId.value) {
      headers['Mcp-Session-Id'] = sessionId.value;
    } else {
      headers['Mcp-Protocol-Version'] = MCP_PROTOCOL_VERSION;
    }

    return headers;
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

  function createHttpError(method: string, status: number): Error {
    return new Error(`${method} failed: HTTP ${status}`);
  }

  async function sendRequest(method: string, request: JsonRpcRequest): Promise<Response> {
    try {
      const httpFetch = getHttpFetch();
      return await httpFetch(mcpUrl, {
        method: 'POST',
        headers: buildHeaders(),
        body: JSON.stringify(request),
      });
    } catch (err: unknown) {
      const message = getErrorMessage(err);
      error.value = `Network error during ${method}: ${message}`;
      throw new Error(error.value);
    }
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

  async function jsonRpc<T = unknown>(
    method: string,
    params?: unknown,
    options: { notification?: boolean; allowReconnect?: boolean } = {},
  ): Promise<{ response: Response; result?: T }> {
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
    };

    if (!options.notification) {
      request.id = nextId++;
    }

    async function execute(canReconnect: boolean): Promise<{ response: Response; result?: T }> {
      const response = await sendRequest(method, request);

      if (response.status === 404 && canReconnect && options.allowReconnect !== false) {
        const originalError = createHttpError(method, response.status);
        error.value = originalError.message;

        try {
          await reconnectSession();
        } catch (reconnectError: unknown) {
          if (reconnectError instanceof Error) {
            throw reconnectError;
          }

          throw new Error(getErrorMessage(reconnectError));
        }

        return execute(false);
      }

      if (options.notification) {
        if (response.status !== 202) {
          error.value = createHttpError(method, response.status).message;
          throw new Error(error.value);
        }

        return { response };
      }

      if (!response.ok) {
        error.value = createHttpError(method, response.status).message;
        throw new Error(error.value);
      }

      let payload: JsonRpcResponse<T>;
      try {
        payload = await parseJsonRpcResponse<T>(response, method, request.id, (notification) => {
          notificationListeners.forEach((listener) => {
            listener(notification);
          });
        });
      } catch (err: unknown) {
        error.value = getErrorMessage(err);
        throw new Error(error.value);
      }

      if (payload.error) {
        error.value = payload.error.message || `JSON-RPC error during ${method}`;
        throw new Error(error.value);
      }

      return { response, result: payload.result };
    }

    return execute(true);
  }

  async function connect(options: { isReconnect?: boolean } = {}): Promise<void> {
    error.value = null;
    resetState();

    try {
      const { response, result } = await jsonRpc<InitializeResult>('initialize', {
        protocolVersion: MCP_PROTOCOL_VERSION,
        capabilities: {},
        clientInfo: { name: 'pipeline-ui', version: '0.1.0' },
      }, { allowReconnect: false });

      const headerSessionId = response.headers.get('Mcp-Session-Id');
      sessionId.value = headerSessionId;
      serverCapabilities.value = result?.capabilities ?? null;

      try {
        await jsonRpc('notifications/initialized', undefined, {
          notification: true,
          allowReconnect: false,
        });
      } catch (err: unknown) {
        const message = `Initialized notification failed: ${getErrorMessage(err)}`;
        error.value = message;
        resetState();
        throw new Error(message);
      }

      connected.value = true;
      reconnecting.value = false;
      error.value = null;
    } catch (err: unknown) {
      const message = error.value ?? getErrorMessage(err);
      resetState();
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
    const currentSessionId = sessionId.value;
    if (!currentSessionId) {
      reconnecting.value = false;
      reconnectPromise = null;
      resetState();
      error.value = null;
      return;
    }

    try {
      const httpFetch = getHttpFetch();
      await httpFetch(mcpUrl, {
        method: 'DELETE',
        headers: {
          Accept: 'application/json, text/event-stream',
          'Mcp-Session-Id': currentSessionId,
        },
      });
    } catch {
      // Best-effort teardown; swallow network errors.
    } finally {
      reconnecting.value = false;
      reconnectPromise = null;
      resetState();
      error.value = null;
    }
  }

  async function ensureConnectedSession(): Promise<void> {
    if (connected.value) return;
    if (reconnecting.value && reconnectPromise) {
      await reconnectPromise;
    }
    if (!connected.value) {
      error.value = 'MCP client is not connected';
      throw new Error(error.value);
    }
  }

  async function callTool<T = unknown>(name: string, args: Record<string, unknown> = {}): Promise<T> {
    error.value = null;
    await ensureConnectedSession();

    const toolResult = await callToolRaw(name, args);
    const rawText = toolResult.content?.[0]?.text;

    if (toolResult.isError) {
      error.value = rawText || 'Unknown tool error';
      throw new Error(error.value);
    }

    if (typeof rawText !== 'string') {
      return undefined as T;
    }

    try {
      return JSON.parse(rawText) as T;
    } catch {
      return rawText as T;
    }
  }

  async function callToolRaw(name: string, args: Record<string, unknown> = {}): Promise<ToolCallResult> {
    // TODO(T-218 follow-up): split this composable; it intentionally exceeds the
    // preferred LOC limit until MCP client transport helpers are extracted.
    error.value = null;
    await ensureConnectedSession();

    const { result } = await jsonRpc<ToolCallResult>('tools/call', {
      name,
      arguments: args,
    });

    return result ?? { content: [] };
  }

  async function listTools(): Promise<ToolDefinition[]> {
    error.value = null;
    await ensureConnectedSession();
  
    const { result } = await jsonRpc<ToolsListResult>('tools/list');
    return (result?.tools ?? []).map((tool) => {
      const rawTool = tool as unknown as Record<string, unknown>;
      return {
        ...tool,
        appUi: extractAppUiMetadata(rawTool),
      };
    });
  }

  async function readResource(uri: string): Promise<ResourceContent> {
    error.value = null;
    await ensureConnectedSession();

    const { result } = await jsonRpc<ResourcesReadResult>('resources/read', { uri });
    return result?.contents?.[0] ?? { uri };
  }

  async function subscribeResource(uri: string): Promise<void> {
    error.value = null;
    await ensureConnectedSession();
    await jsonRpc('resources/subscribe', { uri });
  }

  async function unsubscribeResource(uri: string): Promise<void> {
    error.value = null;
    await ensureConnectedSession();
    await jsonRpc('resources/unsubscribe', { uri });
  }

  async function listResources(): Promise<ResourceDescriptor[]> {
    error.value = null;
    await ensureConnectedSession();
    const { result } = await jsonRpc<ResourcesListResult>('resources/list');
    return result?.resources ?? [];
  }

  async function listResourceTemplates(): Promise<ResourceTemplateDescriptor[]> {
    error.value = null;
    await ensureConnectedSession();
    const { result } = await jsonRpc<ResourceTemplatesListResult>('resources/templates/list');
    return result?.resourceTemplates ?? [];
  }

  function supportsResourceSubscribe(): boolean {
    return serverCapabilities.value?.resources?.subscribe === true;
  }

  function onNotification(handler: (notification: JsonRpcNotification) => void): () => void {
    notificationListeners.add(handler);
    return () => {
      notificationListeners.delete(handler);
    };
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
    callTool,
    callToolRaw,
    listTools,
    readResource,
    subscribeResource,
    unsubscribeResource,
    listResources,
    listResourceTemplates,
    supportsResourceSubscribe,
    onNotification,
    onDisconnect,
  };
}
