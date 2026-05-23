import { Client } from '@modelcontextprotocol/sdk/client/index';
import { StreamableHTTPClientTransport, StreamableHTTPError } from '@modelcontextprotocol/sdk/client/streamableHttp';
import type { CompleteResult, GetPromptResult, Notification, Prompt as PromptDescriptor, PromptReference, ResourceTemplateReference } from '@modelcontextprotocol/sdk/types';
import { LoggingMessageNotificationSchema, ProgressNotificationSchema, PromptListChangedNotificationSchema, ResourceListChangedNotificationSchema, ResourceUpdatedNotificationSchema, ToolListChangedNotificationSchema } from '@modelcontextprotocol/sdk/types';
import { MCP_PROTOCOL_VERSION } from '../lib/constants';
import { sanitizeResourceErrorMessage, warnRawResourceErrorInDev } from '../lib/error-sanitizer';
import { getHttpFetch } from '../lib/http';
import { extractAppUiMetadata } from '../stores/agent-capabilities-helpers';
import type { JsonRpcNotification, ResourceContent, ResourceDescriptor, ResourceTemplateDescriptor, ToolCallResult, ToolDefinition } from './mcpTransport';
import { getErrorMessage } from './mcpTransport';
import { ToolEnvelopeError } from './pipeline-mcp-transport';

export type SdkClient = Client;
export type SdkTransport = StreamableHTTPClientTransport;

export interface McpServerCapabilities {
  completions?: object;
  prompts?: { listChanged?: boolean };
  resources?: { subscribe?: boolean; listChanged?: boolean };
  [key: string]: unknown;
}

export interface NotificationRouter {
  dispatch(notification: Notification | JsonRpcNotification): void;
  onNotification(handler: (notification: JsonRpcNotification) => void): () => void;
}

function getSanitizedErrorMessage(error: unknown): string {
  const rawError = getErrorMessage(error);
  warnRawResourceErrorInDev(rawError);
  return sanitizeResourceErrorMessage(rawError);
}

export function createNotificationRouter(): NotificationRouter {
  const notificationListeners = new Set<(notification: JsonRpcNotification) => void>();
  return {
    dispatch(notification) { notificationListeners.forEach((listener) => listener(notification as JsonRpcNotification)); },
    onNotification(handler) { notificationListeners.add(handler); return () => notificationListeners.delete(handler); },
  };
}

export function createSdkMcpClient(options: {
  mcpUrl: string;
  notificationRouter: NotificationRouter;
  onError(error: string): void;
  onClose(): void;
}): { client: SdkClient; transport: SdkTransport } {
  const client = new Client({ name: 'pipeline-ui', version: '0.1.0' }, { capabilities: { elicitation: {} } });
  const transport = new StreamableHTTPClientTransport(new URL(options.mcpUrl), { fetch: getHttpFetch() });
  transport.setProtocolVersion(MCP_PROTOCOL_VERSION);
  client.onerror = (err) => options.onError(getSanitizedErrorMessage(err));
  client.onclose = options.onClose;
  client.fallbackNotificationHandler = async (notification) => options.notificationRouter.dispatch(notification);
  for (const schema of [
    ResourceUpdatedNotificationSchema, ResourceListChangedNotificationSchema, ToolListChangedNotificationSchema,
    ProgressNotificationSchema, LoggingMessageNotificationSchema, PromptListChangedNotificationSchema,
  ]) client.setNotificationHandler(schema, (notification) => options.notificationRouter.dispatch(notification));
  return { client, transport };
}

export function toServerCapabilities(client: SdkClient): McpServerCapabilities | null {
  return (client.getServerCapabilities() ?? null) as McpServerCapabilities | null;
}

export const gatewayPassthroughTools = new Set([
  'GatewayIndex', 'GatewayExecute', 'GatewayBatch', 'GatewayIntent',
  'gate_Create', 'ReadMcpResource', 'GetHelp',
]);

function isSessionExpiredError(err: unknown): boolean {
  return err instanceof StreamableHTTPError && err.code === 404;
}

export function createMcpClientOps(options: {
  ensureConnectedSession(): Promise<SdkClient>;
  reconnectSession(): Promise<void>;
  setError(message: string | null): void;
}) {
  const domainMap = new Map<string, string>();
  const directMcpFallbackTools = new Set(['status']);
  let domainMapBuilt = false;
  let domainMapBuildPromise: Promise<void> | null = null;
  type GatewayIndexErrorKind = 'call' | 'content' | 'json' | 'schema';

  function createGatewayIndexError(message: string, kind: GatewayIndexErrorKind, cause?: unknown): Error {
    const causeMessage = cause ? `: ${getSanitizedErrorMessage(cause)}` : '';
    const err = new Error(`${message}${causeMessage}`) as Error & { cause?: unknown; gatewayIndexErrorKind?: GatewayIndexErrorKind };
    err.gatewayIndexErrorKind = kind;
    if (cause) err.cause = cause;
    return err;
  }

  function getGatewayIndexErrorKind(err: unknown): GatewayIndexErrorKind | null {
    if (typeof err !== 'object' || err === null || !('gatewayIndexErrorKind' in err)) return null;
    const kind = (err as { gatewayIndexErrorKind?: unknown }).gatewayIndexErrorKind;
    return kind === 'call' || kind === 'content' || kind === 'json' || kind === 'schema' ? kind : null;
  }

  function getErrorCause(err: unknown): unknown {
    return typeof err === 'object' && err !== null && 'cause' in err ? (err as { cause?: unknown }).cause : undefined;
  }

  function isSessionExpiredErrorOrCause(err: unknown): boolean {
    return isSessionExpiredError(err) || isSessionExpiredError(getErrorCause(err));
  }

  function formatToolCallResultContent(result: ToolCallResult): string {
    const contentText = result.content?.map((item) => {
      if (typeof item.text === 'string') return item.text;
      try { return JSON.stringify(item); } catch { return String(item); }
    }).filter((item) => item.length > 0).join('; ');
    if (contentText) return contentText;
    if (!result.structuredContent) return 'Unknown GatewayIndex tool error';
    try { return JSON.stringify(result.structuredContent); } catch { return String(result.structuredContent); }
  }

  function parseToolEnvelopeError(rawText: string | undefined): ToolEnvelopeError | null {
    if (!rawText) return null;
    try {
      const parsed = JSON.parse(rawText) as { success?: unknown; error?: { code?: unknown; message?: unknown } };
      if (parsed.success === false && typeof parsed.error?.code === 'string') {
        const message = typeof parsed.error.message === 'string' ? parsed.error.message : rawText;
        return new ToolEnvelopeError(parsed.error.code, message);
      }
    } catch {
      return null;
    }
    return null;
  }

  async function readGatewayIndex(client: SdkClient, args: Record<string, unknown>, context: string): Promise<unknown> {
    let indexResult: ToolCallResult;
    try { indexResult = (await client.callTool({ name: 'GatewayIndex', arguments: args })) as ToolCallResult; }
    catch (err: unknown) { throw createGatewayIndexError(`GatewayIndex failed while loading ${context}`, 'call', err); }
    if (indexResult.isError) {
      throw createGatewayIndexError(`GatewayIndex returned MCP tool error while loading ${context}: ${formatToolCallResultContent(indexResult)}`, 'call');
    }
    const indexContent = indexResult.content?.[0]?.text;
    if (typeof indexContent !== 'string' || indexContent.length === 0) {
      throw createGatewayIndexError(`GatewayIndex returned no JSON content while loading ${context}`, 'content');
    }
    try { return JSON.parse(indexContent) as unknown; }
    catch (err: unknown) { throw createGatewayIndexError(`GatewayIndex returned malformed JSON while loading ${context}`, 'json', err); }
  }

  function namedItems(value: unknown, fieldName: string, context: string): Array<{ name: string }> {
    if (!Array.isArray(value)) throw createGatewayIndexError(`GatewayIndex ${context} must include a ${fieldName} array`, 'schema');
    return value.map((item, index) => {
      if (typeof item === 'object' && item !== null && 'name' in item && typeof item.name === 'string' && item.name.length > 0) {
        return { name: item.name };
      }
      throw createGatewayIndexError(`GatewayIndex ${context} has malformed ${fieldName} entry at index ${index}`, 'schema');
    });
  }

  async function buildDomainMap(): Promise<void> {
    const client = await options.ensureConnectedSession();
    const indexData = await readGatewayIndex(client, {}, 'gateway domain index');
    const domains = namedItems((indexData as { domains?: unknown }).domains, 'domains', 'domain index');
    const nextMap = new Map<string, string>();
    await Promise.all(domains.map(async (domain) => {
      const context = `domain ${domain.name}`;
      const domainData = await readGatewayIndex(client, { domain: domain.name }, `gateway ${context}`);
      const commands = namedItems((domainData as { commands?: unknown }).commands, 'commands', context);
      for (const command of commands) if (!gatewayPassthroughTools.has(command.name)) nextMap.set(command.name, domain.name);
    }));
    domainMap.clear();
    for (const [toolName, domainName] of nextMap) domainMap.set(toolName, domainName);
    domainMapBuilt = true;
  }

  async function ensureDomainMap(): Promise<void> {
    if (domainMapBuilt) return;
    if (!domainMapBuildPromise) domainMapBuildPromise = buildDomainMap().finally(() => { domainMapBuildPromise = null; });
    await domainMapBuildPromise;
  }

  async function ensureDomainMapWithReconnect(): Promise<void> {
    try { await ensureDomainMap(); }
    catch (err: unknown) {
      if (!isSessionExpiredErrorOrCause(err)) throw err;
      options.setError('MCP session expired: HTTP 404');
      await options.reconnectSession();
      domainMapBuilt = false;
      domainMapBuildPromise = null;
      await ensureDomainMap();
    }
  }

  async function withReconnect<T>(operation: (client: SdkClient) => Promise<T>): Promise<T> {
    try { return await operation(await options.ensureConnectedSession()); }
    catch (err: unknown) {
      if (!isSessionExpiredError(err)) { options.setError(getSanitizedErrorMessage(err)); throw err; }
      options.setError('MCP session expired: HTTP 404');
      await options.reconnectSession();
      domainMapBuilt = false;
      await buildDomainMap();
      try { return await operation(await options.ensureConnectedSession()); }
      catch (retryErr: unknown) { options.setError(getSanitizedErrorMessage(retryErr)); throw retryErr; }
    }
  }

  async function callTool<T = unknown>(name: string, args: Record<string, unknown> = {}): Promise<T> {
    options.setError(null);
    const toolResult = await callToolRaw(name, args);
    const rawText = toolResult.content?.[0]?.text;
    if (toolResult.isError) {
      const envelopeError = parseToolEnvelopeError(rawText);
      options.setError(envelopeError?.message ?? rawText ?? 'Unknown tool error');
      throw envelopeError ?? new Error(rawText || 'Unknown tool error');
    }
    if (typeof rawText !== 'string') return undefined as T;
    try { return JSON.parse(rawText) as T; } catch { return rawText as T; }
  }

  async function callToolRaw(name: string, args: Record<string, unknown> = {}): Promise<ToolCallResult> {
    options.setError(null);
    if (!gatewayPassthroughTools.has(name)) {
      try { await ensureDomainMapWithReconnect(); }
      catch (err: unknown) {
        if (getGatewayIndexErrorKind(err) === 'call' && directMcpFallbackTools.has(name)) {
          return withReconnect(async (client) => (await client.callTool({ name, arguments: args })) as ToolCallResult);
        }
        options.setError(getSanitizedErrorMessage(err));
        throw err;
      }
    }
    return withReconnect(async (client) => {
      const domain = gatewayPassthroughTools.has(name) ? undefined : domainMap.get(name);
      if (domain) return (await client.callTool({ name: 'GatewayExecute', arguments: { domain, command: name, parameters: args } })) as ToolCallResult;
      if (!gatewayPassthroughTools.has(name) && !directMcpFallbackTools.has(name)) {
        throw createGatewayIndexError(`GatewayIndex domain map does not include a route for ${name}; refusing direct MCP fallback without allowlist proof`, 'schema');
      }
      return (await client.callTool({ name, arguments: args })) as ToolCallResult;
    });
  }

  async function listTools(): Promise<ToolDefinition[]> {
    options.setError(null);
    const result = await withReconnect((client) => client.listTools());
    return (result.tools ?? []).map((tool) => ({ ...tool, appUi: extractAppUiMetadata(tool as unknown as Record<string, unknown>) })) as ToolDefinition[];
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

  async function listPrompts(): Promise<PromptDescriptor[]> {
    options.setError(null);
    const result = await withReconnect((client) => client.listPrompts());
    return (result.prompts ?? []) as PromptDescriptor[];
  }

  async function getPrompt(name: string, args?: Record<string, string>): Promise<GetPromptResult> {
    options.setError(null);
    return withReconnect((client) => client.getPrompt({ name, arguments: args }));
  }

  async function complete(ref: PromptReference | ResourceTemplateReference, argName: string, value: string): Promise<CompleteResult> {
    options.setError(null);
    return withReconnect((client) => client.complete({ ref, argument: { name: argName, value } }));
  }

  return {
    callTool, callToolRaw, listTools, readResource, subscribeResource, unsubscribeResource, listResources,
    listResourceTemplates, listPrompts, getPrompt, complete,
  };
}
