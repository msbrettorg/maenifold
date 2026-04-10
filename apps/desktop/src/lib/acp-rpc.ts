import { isRecord, withAcpMeta } from './acp-meta';

export type JsonRpcId = number | string;

export type JsonRpcError = {
  code: number;
  message: string;
};

type JsonRpcRequest = {
  jsonrpc: '2.0';
  id: JsonRpcId;
  method: string;
  params: Record<string, unknown>;
};

type JsonRpcNotification = {
  jsonrpc: '2.0';
  method: string;
  params: Record<string, unknown>;
};

type JsonRpcResponse = {
  jsonrpc: '2.0';
  id: JsonRpcId;
  result?: unknown;
  error?: JsonRpcError;
};

export function createJsonRpcRequest(
  id: JsonRpcId,
  method: string,
  params?: unknown
): JsonRpcRequest {
  return { jsonrpc: '2.0', id, method, params: withAcpMeta(params ?? {}) };
}

export function createJsonRpcNotification(
  method: string,
  params?: unknown
): JsonRpcNotification {
  return { jsonrpc: '2.0', method, params: withAcpMeta(params ?? {}) };
}

export function createJsonRpcSuccessResponse(id: JsonRpcId, result: unknown): JsonRpcResponse {
  return { jsonrpc: '2.0', id, result };
}

export function createJsonRpcErrorResponse(
  id: JsonRpcId,
  error: JsonRpcError
): JsonRpcResponse {
  return { jsonrpc: '2.0', id, error };
}

export function isJsonRpcResponse(parsed: Record<string, unknown>): parsed is JsonRpcResponse {
  return 'id' in parsed && parsed.id !== undefined && !('method' in parsed);
}

export function isJsonRpcRequest(parsed: Record<string, unknown>): parsed is JsonRpcRequest {
  return 'id' in parsed && parsed.id !== undefined && 'method' in parsed;
}

export function isJsonRpcNotification(
  parsed: Record<string, unknown>
): parsed is JsonRpcNotification {
  return !('id' in parsed) && typeof parsed.method === 'string';
}

export function asJsonRpcRecord(parsed: unknown): Record<string, unknown> {
  return isRecord(parsed) ? parsed : {};
}
