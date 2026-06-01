import { MCP_BASE_URL } from './config';

type JsonRpcId = string | number;

const INITIAL_PROTOCOL_VERSION = '2025-06-18';

interface JsonRpcErrorShape {
  code: number;
  message: string;
  data?: unknown;
}

interface JsonRpcResponse<T> {
  jsonrpc: '2.0';
  id?: JsonRpcId | null;
  result?: T;
  error?: JsonRpcErrorShape;
}

interface ResourceContentsItem {
  text?: string;
  [key: string]: unknown;
}

interface ReadResourceResult {
  contents?: ResourceContentsItem[];
  [key: string]: unknown;
}

interface InitializeResult {
  protocolVersion?: string;
  [key: string]: unknown;
}

export class McpHttpError extends Error {
  readonly status?: number;
  readonly statusText?: string;
  readonly code?: number;
  readonly bodyText?: string;
  readonly data?: unknown;

  constructor(
    message: string,
    options: { status?: number; statusText?: string; code?: number; bodyText?: string; data?: unknown } = {}
  ) {
    super(message);
    this.name = 'McpHttpError';
    this.status = options.status;
    this.statusText = options.statusText;
    this.code = options.code;
    this.bodyText = options.bodyText;
    this.data = options.data;
  }
}

function createRequestId(): JsonRpcId {
  if (typeof globalThis.crypto?.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }

  return `mcp-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function parseJsonPayload(jsonText: string): unknown {
  try {
    return JSON.parse(jsonText);
  } catch (error) {
    throw new McpHttpError(
      `Failed to parse MCP response payload: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

function parseSsePayload(bodyText: string): unknown {
  const dataLines: string[] = [];

  for (const line of bodyText.split(/\r?\n/)) {
    if (!line || line.startsWith(':') || line.startsWith('id:') || line.startsWith('retry:')) {
      continue;
    }

    if (line.startsWith('data:')) {
      dataLines.push(line.replace(/^data:\s?/, ''));
    }
  }

  if (dataLines.length === 0) {
    return null;
  }

  return parseJsonPayload(dataLines.join('\n'));
}

function parseTransportPayload(bodyText: string, contentType: string | null): unknown {
  const trimmed = bodyText.trim();
  if (!trimmed) return null;

  const normalizedContentType = contentType?.toLowerCase() ?? '';

  if (normalizedContentType.includes('text/event-stream')) {
    return parseSsePayload(bodyText);
  }

  if (normalizedContentType.includes('application/json')) {
    return parseJsonPayload(trimmed);
  }

  if (trimmed.startsWith('data:') || trimmed.startsWith(':') || trimmed.includes('\ndata:')) {
    return parseSsePayload(bodyText);
  }

  return parseJsonPayload(trimmed);
}

export class McpHttpClient {
  private readonly endpoint: string;
  private sessionId: string | null = null;
  private protocolVersion = INITIAL_PROTOCOL_VERSION;
  private initialized = false;
  private initializePromise: Promise<void> | null = null;

  constructor(baseUrl: string = MCP_BASE_URL) {
    this.endpoint = `${baseUrl.replace(/\/+$/, '')}/mcp`;
  }

  async initialize(): Promise<void> {
    if (this.initialized) return;
    if (this.initializePromise) return this.initializePromise;

    this.initializePromise = (async () => {
      try {
        const result = await this.sendRequest<InitializeResult>(
          'initialize',
          {
            protocolVersion: INITIAL_PROTOCOL_VERSION,
            capabilities: {},
            clientInfo: {
              name: 'maenifold-ui',
              version: '0.1.0',
            },
          },
          { omitProtocolVersionHeader: true }
        );

        if (typeof result.protocolVersion === 'string' && result.protocolVersion.trim()) {
          this.protocolVersion = result.protocolVersion;
        }

        await this.sendNotification('notifications/initialized', {});
        this.initialized = true;
      } catch (error) {
        this.initializePromise = null;
        throw error;
      }
    })();

    return this.initializePromise;
  }

  async readResource<T = unknown>(uri: string): Promise<T> {
    await this.initialize();

    const result = await this.sendRequest<ReadResourceResult>('resources/read', { uri });
    const text = result.contents?.[0]?.text;

    if (typeof text !== 'string') {
      throw new McpHttpError(`MCP resource '${uri}' did not return contents[0].text`);
    }

    try {
      return JSON.parse(text) as T;
    } catch (error) {
      throw new McpHttpError(
        `Failed to parse JSON from MCP resource '${uri}': ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async callTool<T = unknown>(name: string, args: Record<string, unknown>): Promise<T> {
    await this.initialize();

    return this.sendRequest<T>('tools/call', { name, arguments: args });
  }

  private async sendRequest<T>(
    method: string,
    params: unknown,
    options: { omitProtocolVersionHeader?: boolean } = {}
  ): Promise<T> {
    const payload = await this.post(
      {
        jsonrpc: '2.0',
        id: createRequestId(),
        method,
        params,
      },
      options
    );

    const rpcError = extractRpcError(payload);
    if (rpcError) {
      throw new McpHttpError(rpcError.message, {
        code: rpcError.code,
        data: rpcError.data,
      });
    }

    if (!payload || typeof payload !== 'object' || !('result' in payload)) {
      throw new McpHttpError(`MCP method '${method}' returned an invalid JSON-RPC response`);
    }

    return (payload as JsonRpcResponse<T>).result as T;
  }

  private async sendNotification(method: string, params: unknown): Promise<void> {
    const payload = await this.post({
      jsonrpc: '2.0',
      method,
      params,
    });

    const rpcError = extractRpcError(payload);
    if (rpcError) {
      throw new McpHttpError(rpcError.message, {
        code: rpcError.code,
        data: rpcError.data,
      });
    }
  }

  private async post(
    body: Record<string, unknown>,
    options: { omitProtocolVersionHeader?: boolean } = {}
  ): Promise<unknown> {
    const headers = new Headers({
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    });

    if (!options.omitProtocolVersionHeader) {
      headers.set('MCP-Protocol-Version', this.protocolVersion);
    }

    if (this.sessionId) {
      headers.set('mcp-session-id', this.sessionId);
    }

    let response: Response;
    try {
      response = await fetch(this.endpoint, {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
      });
    } catch (error) {
      throw new McpHttpError(`MCP request failed: ${error instanceof Error ? error.message : String(error)}`);
    }

    const nextSessionId = response.headers.get('mcp-session-id');
    if (nextSessionId) {
      this.sessionId = nextSessionId;
    }

    const bodyText = await response.text();

    if (!response.ok) {
      throw new McpHttpError(`MCP HTTP ${response.status} ${response.statusText}`.trim(), {
        status: response.status,
        statusText: response.statusText,
        bodyText,
        data: bodyText,
      });
    }

    return parseTransportPayload(bodyText, response.headers.get('content-type'));
  }
}

function extractRpcError(payload: unknown): JsonRpcErrorShape | null {
  if (!payload || typeof payload !== 'object' || !('error' in payload)) {
    return null;
  }

  const candidate = (payload as { error?: unknown }).error;
  if (!candidate || typeof candidate !== 'object') {
    return null;
  }

  const maybeError = candidate as Partial<JsonRpcErrorShape>;
  if (typeof maybeError.message !== 'string' || typeof maybeError.code !== 'number') {
    return null;
  }

  return {
    code: maybeError.code,
    message: maybeError.message,
    data: maybeError.data,
  };
}

export const mcpHttpClient = new McpHttpClient();
