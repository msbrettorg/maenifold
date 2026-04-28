import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import type { AddressInfo } from 'node:net';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import {
  CallToolRequestSchema,
  isInitializeRequest,
  ListResourceTemplatesRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
  SubscribeRequestSchema,
  UnsubscribeRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';

interface LoopbackResource {
  uri: string;
  mimeType?: string;
  text: string;
}

interface RequestRecord {
  method: string;
  url: string;
  headers: IncomingMessage['headers'];
  body: unknown;
}

interface LoopbackOptions {
  stateless?: boolean;
  resourcesSubscribe?: boolean;
  resources?: LoopbackResource[];
  failOnce?: (request: RequestRecord) => boolean;
}

interface LoopbackServer {
  baseUrl: string;
  requests: RequestRecord[];
  notifyOnNextList(uri: string): void;
  close(): Promise<void>;
}

function readRequestBody(req: IncomingMessage): Promise<unknown> {
  if (req.method !== 'POST') {
    return Promise.resolve(undefined);
  }

  return new Promise((resolve, reject) => {
    let raw = '';
    req.setEncoding('utf8');
    req.on('data', (chunk) => {
      raw += chunk;
    });
    req.on('error', reject);
    req.on('end', () => {
      if (!raw) {
        resolve(undefined);
        return;
      }

      try {
        resolve(JSON.parse(raw) as unknown);
      } catch (error) {
        reject(error);
      }
    });
  });
}

function createConfiguredMcpServer(options: LoopbackOptions, pendingListNotifications: string[]): McpServer {
  const server = new McpServer(
    { name: 'maenifold-loopback-test', version: '0.0.0' },
    {
      capabilities: {
        tools: { listChanged: false },
        resources: {
          listChanged: true,
          ...(options.resourcesSubscribe === false ? {} : { subscribe: true }),
        },
      },
    },
  );

  const resources = options.resources ?? [];
  const subscribedUris = new Set<string>();

  server.server.setRequestHandler(ListToolsRequestSchema, () => ({
    tools: [
      { name: 'GatewayIndex', inputSchema: { type: 'object' } },
      { name: 'GatewayExecute', inputSchema: { type: 'object' } },
      { name: 'Pipeline', inputSchema: { type: 'object' } },
    ],
  }));

  server.server.setRequestHandler(CallToolRequestSchema, (request) => {
    const name = request.params.name;
    const args = (request.params.arguments ?? {}) as Record<string, unknown>;

    if (name === 'GatewayIndex') {
      const result = args.domain === 'maenifold.workflow'
        ? { commands: [{ name: 'Pipeline' }] }
        : { domains: [{ name: 'maenifold.workflow' }] };
      return {
        content: [{ type: 'text', text: JSON.stringify(result) }],
      };
    }

    if (name === 'GatewayExecute') {
      const command = args.command;
      const result = command === 'Pipeline'
        ? { success: true, data: { pipelines: [] } }
        : { success: false, error: `unexpected command: ${String(command)}` };
      return {
        content: [{ type: 'text', text: JSON.stringify(result) }],
      };
    }

    if (name === 'Pipeline') {
      return {
        content: [{ type: 'text', text: JSON.stringify({ success: true, data: { pipelines: [] } }) }],
      };
    }

    return {
      isError: true,
      content: [{ type: 'text', text: `Unknown tool: ${name}` }],
    };
  });

  server.server.setRequestHandler(ReadResourceRequestSchema, (request) => {
    const resource = resources.find((item) => item.uri === request.params.uri);
    return {
      contents: resource
        ? [{ uri: resource.uri, mimeType: resource.mimeType, text: resource.text }]
        : [{ uri: request.params.uri, text: '' }],
    };
  });

  server.server.setRequestHandler(ListResourcesRequestSchema, async (_request, extra) => {
    const uri = pendingListNotifications.shift();
    if (uri) {
      await extra.sendNotification({
        method: 'notifications/resources/updated',
        params: { uri },
      });
    }

    return {
      resources: resources.map((resource) => ({
        uri: resource.uri,
        name: resource.uri,
        mimeType: resource.mimeType,
      })),
    };
  });

  server.server.setRequestHandler(ListResourceTemplatesRequestSchema, () => ({
    resourceTemplates: [],
  }));

  server.server.setRequestHandler(SubscribeRequestSchema, (request) => {
    subscribedUris.add(request.params.uri);
    return {};
  });

  server.server.setRequestHandler(UnsubscribeRequestSchema, (request) => {
    subscribedUris.delete(request.params.uri);
    return {};
  });

  return server;
}

async function sendJsonRpcError(res: ServerResponse, status: number, message: string): Promise<void> {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(JSON.stringify({
    jsonrpc: '2.0',
    error: { code: -32000, message },
    id: null,
  }));
}

export async function startMcpLoopbackServer(options: LoopbackOptions = {}): Promise<LoopbackServer> {
  const requests: RequestRecord[] = [];
  const pendingListNotifications: string[] = [];
  const transports = new Map<string, StreamableHTTPServerTransport>();
  const mcpServers = new Set<McpServer>();
  let failedOnce = false;
  let nextSessionNumber = 1;

  const httpServer = createServer(async (req, res) => {
    const body = await readRequestBody(req);
    const record: RequestRecord = {
      method: req.method ?? '',
      url: req.url ?? '',
      headers: req.headers,
      body,
    };
    requests.push(record);

    if (options.failOnce && !failedOnce && options.failOnce(record)) {
      failedOnce = true;
      await sendJsonRpcError(res, 404, 'Session not found');
      return;
    }

    if (record.url !== '/mcp') {
      await sendJsonRpcError(res, 404, 'Not found');
      return;
    }

    if (options.stateless) {
      if (record.method !== 'POST') {
        await sendJsonRpcError(res, 405, 'Method not allowed');
        return;
      }

      const mcpServer = createConfiguredMcpServer(options, pendingListNotifications);
      mcpServers.add(mcpServer);
      const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
      await mcpServer.connect(transport);
      await transport.handleRequest(req, res, body);
      res.on('close', () => {
        void transport.close();
        void mcpServer.close();
        mcpServers.delete(mcpServer);
      });
      return;
    }

    const sessionId = typeof req.headers['mcp-session-id'] === 'string'
      ? req.headers['mcp-session-id']
      : undefined;
    const existingTransport = sessionId ? transports.get(sessionId) : undefined;

    if (existingTransport) {
      await existingTransport.handleRequest(req, res, body);
      return;
    }

    if (record.method === 'POST' && !sessionId && isInitializeRequest(body)) {
      const mcpServer = createConfiguredMcpServer(options, pendingListNotifications);
      mcpServers.add(mcpServer);
      let transport: StreamableHTTPServerTransport;
      transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: () => `session-${nextSessionNumber++}`,
        onsessioninitialized: (newSessionId) => {
          transports.set(newSessionId, transport);
        },
      });
      transport.onclose = () => {
        const closedSessionId = transport.sessionId;
        if (closedSessionId) {
          transports.delete(closedSessionId);
        }
        mcpServers.delete(mcpServer);
      };
      await mcpServer.connect(transport);
      await transport.handleRequest(req, res, body);
      return;
    }

    await sendJsonRpcError(res, 400, 'Invalid or missing session ID');
  });

  await new Promise<void>((resolve, reject) => {
    httpServer.once('error', reject);
    httpServer.listen(0, '127.0.0.1', () => {
      httpServer.off('error', reject);
      resolve();
    });
  });

  const address = httpServer.address() as AddressInfo;

  return {
    baseUrl: `http://127.0.0.1:${address.port}`,
    requests,
    notifyOnNextList(uri: string) {
      pendingListNotifications.push(uri);
    },
    async close() {
      await Promise.all(Array.from(mcpServers, (server) => server.close().catch(() => undefined)));
      await Promise.all(Array.from(transports.values(), (transport) => transport.close().catch(() => undefined)));
      await new Promise<void>((resolve, reject) => {
        httpServer.close((error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
    },
  };
}
