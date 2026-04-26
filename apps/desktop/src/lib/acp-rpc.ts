import type {
  AnyNotification as SdkJsonRpcNotification,
  AnyRequest as SdkJsonRpcRequest,
  AnyResponse as SdkJsonRpcResponse,
  ErrorResponse as SdkJsonRpcError,
} from '@agentclientprotocol/sdk/dist/jsonrpc.js';
import { isRecord } from './acp-meta';

export type JsonRpcId = Exclude<SdkJsonRpcRequest['id'], null>;
export type JsonRpcError = SdkJsonRpcError;
export type JsonRpcRequest = SdkJsonRpcRequest & { id: JsonRpcId };
export type JsonRpcNotification = SdkJsonRpcNotification;
export type JsonRpcResponse = SdkJsonRpcResponse & { id: JsonRpcId };

// SDK parity gap: @agentclientprotocol/sdk exports JSON-RPC envelope types and
// connection classes, but not small runtime guards for classifying already-parsed
// traffic monitor messages.
export function isJsonRpcResponse(parsed: Record<string, unknown>): parsed is JsonRpcResponse {
  return 'id' in parsed && parsed.id !== undefined && parsed.id !== null && !('method' in parsed);
}

export function isJsonRpcRequest(parsed: Record<string, unknown>): parsed is JsonRpcRequest {
  return 'id' in parsed && parsed.id !== undefined && parsed.id !== null && 'method' in parsed;
}

export function isJsonRpcNotification(
  parsed: Record<string, unknown>
): parsed is JsonRpcNotification {
  return !('id' in parsed) && typeof parsed.method === 'string';
}

export function asJsonRpcRecord(parsed: unknown): Record<string, unknown> {
  return isRecord(parsed) ? parsed : {};
}
