import { Client } from '@modelcontextprotocol/sdk/client/index';
import {
  StreamableHTTPClientTransport,
  StreamableHTTPError,
} from '@modelcontextprotocol/sdk/client/streamableHttp';
import type { Notification } from '@modelcontextprotocol/sdk/types';
import {
  LoggingMessageNotificationSchema,
  ProgressNotificationSchema,
  ResourceListChangedNotificationSchema,
  ResourceUpdatedNotificationSchema,
  ToolListChangedNotificationSchema,
} from '@modelcontextprotocol/sdk/types';
import { MCP_PROTOCOL_VERSION } from '../lib/constants';
import { getHttpFetch } from '../lib/http';
import { extractAppUiMetadata } from '../stores/agent-capabilities-helpers';
import type {
  JsonRpcNotification,
  ResourceContent,
  ResourceDescriptor,
  ResourceTemplateDescriptor,
  ToolCallResult,
  ToolDefinition,
} from './mcpTransport';
import { getErrorMessage } from './mcpTransport';

export type SdkClient = Client;
export type SdkTransport = StreamableHTTPClientTransport;

export interface McpServerCapabilities {
  resources?: {
    subscribe?: boolean;
    listChanged?: boolean;
  };
  [key: string]: unknown;
}

export interface NotificationRouter {
  dispatch(notification: Notification | JsonRpcNotification): void;
  onNotification(handler: (notification: JsonRpcNotification) => void): () => void;
}

export function createNotificationRouter(): NotificationRouter {
  const notificationListeners = new Set<(notification: JsonRpcNotification) => void>();

  return {
    dispatch(notification) {
      notificationListeners.forEach((listener) => {
        listener(notification as JsonRpcNotification);
      });
    },
    onNotification(handler) {
      notificationListeners.add(handler);
      return () => {
        notificationListeners.delete(handler);
      };
    },
  };
}

export function createSdkMcpClient(options: {
  mcpUrl: string;
  notificationRouter: NotificationRouter;
  onError(error: string): void;
  onClose(): void;
}): { client: SdkClient; transport: SdkTransport } {
  const client = new Client(
    { name: 'pipeline-ui', version: '0.1.0' },
    { capabilities: {} },
  );
  const transport = new StreamableHTTPClientTransport(new URL(options.mcpUrl), {
    fetch: getHttpFetch(),
  });

  // Preserve the previous renderer contract: the initial request carries the
  // configured protocol version until the SDK receives the negotiated version.
  transport.setProtocolVersion(MCP_PROTOCOL_VERSION);

  client.onerror = (err) => {
    options.onError(getErrorMessage(err));
  };
  client.onclose = options.onClose;
  client.fallbackNotificationHandler = async (notification) => {
    options.notificationRouter.dispatch(notification);
  };

  for (const schema of [
    ResourceUpdatedNotificationSchema,
    ResourceListChangedNotificationSchema,
    ToolListChangedNotificationSchema,
    ProgressNotificationSchema,
    LoggingMessageNotificationSchema,
  ]) {
    client.setNotificationHandler(schema, (notification) => {
      options.notificationRouter.dispatch(notification);
    });
  }

  return { client, transport };
}

export function toServerCapabilities(client: SdkClient): McpServerCapabilities | null {
  return (client.getServerCapabilities() ?? null) as McpServerCapabilities | null;
}

function isSessionExpiredError(err: unknown): boolean {
  return err instanceof StreamableHTTPError && err.code === 404;
}

export function createMcpClientOps(options: {
  ensureConnectedSession(): Promise<SdkClient>;
  reconnectSession(): Promise<void>;
  setError(message: string | null): void;
}) {
  const domainMap = new Map<string, string>();
  const gatewayPassthroughTools = new Set(['GetHelp', 'ReadMcpResource', 'GatewayIndex', 'GatewayExecute']);
  let domainMapBuilt = false;

  async function buildDomainMap(): Promise<void> {
    try {
      const client = await options.ensureConnectedSession();
      const indexResult = await client.callTool({ name: 'GatewayIndex', arguments: {} });
      const indexContent = (indexResult as ToolCallResult).content?.[0]?.text;
      if (!indexContent) {
        domainMap.clear();
        domainMapBuilt = true;
        return;
      }
      const indexData = JSON.parse(indexContent);
      const domains: Array<{ name: string }> = indexData.domains ?? [];

      domainMap.clear();

      await Promise.all(domains.map(async (domain) => {
        const domainResult = await client.callTool({
          name: 'GatewayIndex',
          arguments: { domain: domain.name },
        });
        const domainContent = (domainResult as ToolCallResult).content?.[0]?.text;
        if (!domainContent) return;
        const domainData = JSON.parse(domainContent);
        const commands: Array<{ name: string }> = domainData.commands ?? [];
        for (const command of commands) {
          if (!gatewayPassthroughTools.has(command.name)) {
            domainMap.set(command.name, domain.name);
          }
        }
      }));

      domainMapBuilt = true;
    } catch {
      // GatewayIndex is unavailable when gateway routing is not active. In that
      // mode, keep the map empty and continue sending bare tool names.
      domainMap.clear();
      domainMapBuilt = true;
    }
  }

  async function withReconnect<T>(operation: (client: SdkClient) => Promise<T>): Promise<T> {
    try {
      return await operation(await options.ensureConnectedSession());
    } catch (err: unknown) {
      if (!isSessionExpiredError(err)) {
        options.setError(getErrorMessage(err));
        throw err;
      }

      options.setError('MCP session expired: HTTP 404');
      await options.reconnectSession();
      domainMapBuilt = false;
      await buildDomainMap();

      try {
        return await operation(await options.ensureConnectedSession());
      } catch (retryErr: unknown) {
        options.setError(getErrorMessage(retryErr));
        throw retryErr;
      }
    }
  }

  async function callTool<T = unknown>(name: string, args: Record<string, unknown> = {}): Promise<T> {
    options.setError(null);
    const toolResult = await callToolRaw(name, args);
    const rawText = toolResult.content?.[0]?.text;

    if (toolResult.isError) {
      options.setError(rawText || 'Unknown tool error');
      throw new Error(rawText || 'Unknown tool error');
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
    options.setError(null);
    if (!domainMapBuilt) {
      await buildDomainMap();
    }
    return withReconnect(async (client) => {
      const domain = gatewayPassthroughTools.has(name) ? undefined : domainMap.get(name);
      if (domain) {
        return (await client.callTool({
          name: 'GatewayExecute',
          arguments: { domain, command: name, parameters: args },
        })) as ToolCallResult;
      }
      return (await client.callTool({ name, arguments: args })) as ToolCallResult;
    });
  }

  async function listTools(): Promise<ToolDefinition[]> {
    options.setError(null);
    const result = await withReconnect((client) => client.listTools());
    return (result.tools ?? []).map((tool) => ({
      ...tool,
      appUi: extractAppUiMetadata(tool as unknown as Record<string, unknown>),
    })) as ToolDefinition[];
  }

  async function readResource(uri: string): Promise<ResourceContent> {
    options.setError(null);
    const result = await withReconnect((client) => client.readResource({ uri }));
    return (result.contents?.[0] ?? { uri }) as ResourceContent;
  }

  async function subscribeResource(uri: string): Promise<void> {
    options.setError(null);
    await withReconnect((client) => client.subscribeResource({ uri }));
  }

  async function unsubscribeResource(uri: string): Promise<void> {
    options.setError(null);
    await withReconnect((client) => client.unsubscribeResource({ uri }));
  }

  async function listResources(): Promise<ResourceDescriptor[]> {
    options.setError(null);
    const result = await withReconnect((client) => client.listResources());
    return (result.resources ?? []) as ResourceDescriptor[];
  }

  async function listResourceTemplates(): Promise<ResourceTemplateDescriptor[]> {
    options.setError(null);
    const result = await withReconnect((client) => client.listResourceTemplates());
    return (result.resourceTemplates ?? []) as ResourceTemplateDescriptor[];
  }

  return {
    callTool,
    callToolRaw,
    listTools,
    readResource,
    subscribeResource,
    unsubscribeResource,
    listResources,
    listResourceTemplates,
  };
}
