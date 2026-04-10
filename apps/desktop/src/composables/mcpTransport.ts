/**
 * JSON-RPC 2.0 transport types and response-parsing helpers for MCP
 * Streamable HTTP transport.
 *
 * Extracted from useMcpClient to keep composable files ≤ 300 LOC (T-65.15).
 */

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  method: string;
  id?: number;
  params?: unknown;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

export interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id?: number | string | null;
  result?: T;
  error?: JsonRpcError;
}

// ---------------------------------------------------------------------------
// Tool-call result types
// ---------------------------------------------------------------------------

export interface ToolContent {
  type?: string;
  text?: string;
}

export interface ToolCallResult {
  content?: ToolContent[];
  isError?: boolean;
}

export interface ToolDefinition {
  name: string;
  description?: string;
  inputSchema?: unknown;
}

export interface ToolsListResult {
  tools?: ToolDefinition[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isJsonRpcResponse(value: unknown): value is JsonRpcResponse {
  return typeof value === 'object' && value !== null && (value as JsonRpcResponse).jsonrpc === '2.0';
}

function parseSseJsonRpcResponse<T>(body: string, requestId?: number): JsonRpcResponse<T> {
  const eventBlocks = body.split(/\r?\n\r?\n/);

  for (const block of eventBlocks) {
    const dataLines = block
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.startsWith('data:'))
      .map((line) => line.slice(5).trimStart())
      .filter((line) => line.length > 0);

    if (dataLines.length === 0) {
      continue;
    }

    const candidate = dataLines.join('\n');

    try {
      const parsed = JSON.parse(candidate) as unknown;
      if (!isJsonRpcResponse(parsed)) {
        continue;
      }

      if (requestId === undefined || parsed.id === requestId) {
        return parsed as JsonRpcResponse<T>;
      }
    } catch {
      continue;
    }
  }

  const idSuffix = requestId === undefined ? '' : ` for request id ${requestId}`;
  throw new Error(`No valid JSON-RPC response found in SSE stream${idSuffix}`);
}

export async function parseJsonRpcResponse<T>(
  response: Response,
  method: string,
  requestId?: number,
): Promise<JsonRpcResponse<T>> {
  const contentType = response.headers.get('content-type')?.toLowerCase() ?? '';

  if (contentType.includes('text/event-stream')) {
    const body = await response.text();
    try {
      return parseSseJsonRpcResponse<T>(body, requestId);
    } catch (error) {
      const message = getErrorMessage(error);
      throw new Error(`Invalid SSE response from ${method}: ${message}`);
    }
  }

  try {
    return (await response.json()) as JsonRpcResponse<T>;
  } catch {
    throw new Error(`Invalid JSON response from ${method}`);
  }
}
