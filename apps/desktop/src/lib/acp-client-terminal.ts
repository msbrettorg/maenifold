import { AGENT_METHODS, type AnyMessage, type CreateTerminalRequest, type CreateTerminalResponse, type KillTerminalRequest, type KillTerminalResponse, type ReleaseTerminalRequest, type ReleaseTerminalResponse, type SessionNotification, type Stream, type TerminalOutputRequest, type TerminalOutputResponse, type WaitForTerminalExitRequest, type WaitForTerminalExitResponse } from '@agentclientprotocol/sdk';
import type { WritePermissionPolicy } from './acp-file-system';
import { withAcpMeta } from './acp-meta';
import { asJsonRpcRecord, isJsonRpcNotification, isJsonRpcRequest, isJsonRpcResponse, type JsonRpcId } from './acp-rpc';
import type { AcpClientSinks } from './acp-sinks';

type CreateTerminalResponseWithLimit = CreateTerminalResponse & { outputByteLimit?: number };
type TerminalBridge = { sinks: AcpClientSinks };
type TerminalRequestBridge = TerminalBridge & { sendRequest<T>(method: string, params?: unknown): Promise<T> };
type TerminalRequestOnlyBridge = { sendRequest<T>(method: string, params?: unknown): Promise<T> };
type TimeoutRequest = { reject: (error: Error) => void };
type TransportBridge = { agentName: string; pendingMethods: Map<JsonRpcId, string>; timeoutHandles: Map<number, ReturnType<typeof setTimeout>>; pendingRequests: Map<number, TimeoutRequest>; activeRelaySessionId?: string | null; sinks: AcpClientSinks; getOnSessionUpdate: () => ((notification: SessionNotification) => void) | null; extNotification(method: string, params: Record<string, unknown>): Promise<void> };

export function createRejectAllWritesPolicy(): WritePermissionPolicy {
  return { currentModeId: () => null, notifyPlanModeBlocked: () => {} };
}
export function requireNonBlank(value: string, fieldName: string): void {
  if (typeof value !== 'string' || value.trim().length === 0) throw new Error(`terminal request requires ${fieldName}`);
}
export function validateExitStatus(response: WaitForTerminalExitResponse): WaitForTerminalExitResponse {
  if ((response.exitCode != null && typeof response.exitCode !== 'number') || (response.signal != null && typeof response.signal !== 'string')) {
    throw new Error('Invalid terminal exit response');
  }
  return { exitCode: response.exitCode ?? null, signal: response.signal ?? null };
}
export function validateTerminalOutput(response: TerminalOutputResponse): TerminalOutputResponse {
  if (typeof response.output !== 'string' || typeof response.truncated !== 'boolean') {
    throw new Error('Invalid terminal output response');
  }
  if (
    response.exitStatus &&
    ((response.exitStatus.exitCode != null && typeof response.exitStatus.exitCode !== 'number') ||
      (response.exitStatus.signal != null && typeof response.exitStatus.signal !== 'string'))
  ) {
    throw new Error('Invalid terminal output exit status');
  }
  return response;
}
export async function createTerminalImpl(bridge: TerminalBridge, params: CreateTerminalRequest): Promise<CreateTerminalResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.command, 'command');
  const api = window.electronAPI?.terminal?.create;
  if (!api) throw new Error('Terminal API is unavailable');
  const response = await api(params);
  bridge.sinks.terminals.setTerminal(response.terminalId, {
    terminalId: response.terminalId,
    output: '',
    truncated: false,
    released: false,
    exitStatus: null,
    outputByteLimit: response.outputByteLimit,
  });
  return response;
}
export async function terminalOutputImpl(bridge: TerminalRequestBridge, params: TerminalOutputRequest): Promise<TerminalOutputResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  const response = validateTerminalOutput(await bridge.sendRequest<TerminalOutputResponse>('terminal/output', params));
  bridge.sinks.terminals.setTerminal(params.terminalId, {
    terminalId: params.terminalId,
    output: response.output,
    truncated: response.truncated,
    released: false,
    exitStatus: response.exitStatus ?? null,
  });
  return response;
}
export async function waitForTerminalExitImpl(
  bridge: TerminalBridge,
  params: WaitForTerminalExitRequest
): Promise<WaitForTerminalExitResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  const api = window.electronAPI?.terminal?.waitForExit;
  if (!api) throw new Error('Terminal API is unavailable');
  const response = validateExitStatus(await api(params));
  bridge.sinks.terminals.markExited(params.terminalId, response);
  return response;
}
export async function killTerminalImpl(params: KillTerminalRequest): Promise<KillTerminalResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  const api = window.electronAPI?.terminal?.kill;
  if (!api) throw new Error('Terminal API is unavailable');
  return api(params);
}
export async function releaseTerminalImpl(bridge: TerminalBridge, params: ReleaseTerminalRequest): Promise<ReleaseTerminalResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  const api = window.electronAPI?.terminal?.release;
  if (!api) throw new Error('Terminal API is unavailable');
  const response = await api(params);
  bridge.sinks.terminals.markReleased(params.terminalId);
  return response;
}
export async function terminalCreateImpl(bridge: TerminalRequestBridge, params: CreateTerminalRequest): Promise<CreateTerminalResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.command, 'command');
  const response = await bridge.sendRequest<CreateTerminalResponseWithLimit>('terminal/create', params);
  bridge.sinks.terminals.setTerminal(response.terminalId, {
    terminalId: response.terminalId,
    output: '',
    truncated: false,
    released: false,
    exitStatus: null,
    outputByteLimit: response.outputByteLimit,
  });
  return response;
}
export async function terminalWaitForExitImpl(
  bridge: TerminalRequestBridge,
  params: WaitForTerminalExitRequest
): Promise<WaitForTerminalExitResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  const response = validateExitStatus(await bridge.sendRequest<WaitForTerminalExitResponse>('terminal/wait_for_exit', params));
  bridge.sinks.terminals.markExited(params.terminalId, response);
  return response;
}
export async function terminalKillImpl(bridge: TerminalRequestOnlyBridge, params: KillTerminalRequest): Promise<KillTerminalResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  return bridge.sendRequest<KillTerminalResponse>('terminal/kill', params);
}
export async function terminalReleaseImpl(
  bridge: TerminalRequestBridge,
  params: ReleaseTerminalRequest
): Promise<ReleaseTerminalResponse> {
  requireNonBlank(params.sessionId, 'sessionId');
  requireNonBlank(params.terminalId, 'terminalId');
  const response = await bridge.sendRequest<ReleaseTerminalResponse>('terminal/release', params);
  bridge.sinks.terminals.markReleased(params.terminalId);
  return response;
}
export function createWebSocketStreamImpl(bridge: TransportBridge, socket: WebSocket): Stream {
  let readableClosed = false;
  const closeReadable = (controller: ReadableStreamDefaultController<AnyMessage>) => {
    if (!readableClosed) {
      readableClosed = true;
      controller.close();
    }
  };
  const readable = new ReadableStream<AnyMessage>({
    start: (controller) => {
      socket.addEventListener('message', (event) => {
        if (typeof event.data !== 'string' || readableClosed) return;
        for (const line of event.data.split('\n')) {
          const trimmed = line.trim();
          if (!trimmed) continue;
          try {
            const message = asJsonRpcRecord(JSON.parse(trimmed)) as AnyMessage;
            if (handleAcpUiNotificationImpl(bridge, message)) continue;
            recordTrafficImpl(bridge, 'in', message);
            controller.enqueue(message);
          } catch (error) {
            console.error('Failed to handle ACP relay message:', error);
          }
        }
      });
      socket.addEventListener('close', () => closeReadable(controller));
      socket.addEventListener('error', () => controller.error(new Error(`ACP relay socket errored (${bridge.agentName})`)));
    },
    cancel: () => {
      if (socket.readyState < WebSocket.CLOSING) socket.close(1000, 'stream cancelled');
    },
  });
  const writable = new WritableStream<AnyMessage>({
    write: (message) => {
      if (socket.readyState !== WebSocket.OPEN) throw new Error(`ACP relay socket is not connected (${bridge.agentName})`);
      recordTrafficImpl(bridge, 'out', message);
      socket.send(JSON.stringify(message));
    },
  });
  return { readable, writable };
}
export function handleAcpUiNotificationImpl(bridge: TransportBridge, message: AnyMessage): boolean {
  const parsed = asJsonRpcRecord(message);
  if (!isJsonRpcNotification(parsed)) return false;
  if (parsed.method === 'session/update') {
    recordTrafficImpl(bridge, 'in', message);
    try {
      bridge.getOnSessionUpdate()?.((parsed.params ?? {}) as SessionNotification);
    } catch (error) {
      console.error('Failed to handle ACP session/update notification:', error);
    }
    return true;
  }
  if (parsed.method === '_relay/stderr') {
    recordTrafficImpl(bridge, 'in', message);
    const params = asJsonRpcRecord(parsed.params);
    void bridge.extNotification(parsed.method, params);
    return true;
  }
  return false;
}
export function recordTrafficImpl(bridge: TransportBridge, direction: 'in' | 'out', message: AnyMessage): void {
  const parsed = asJsonRpcRecord(message);
  if (isJsonRpcResponse(parsed)) {
    const id = parsed.id as JsonRpcId;
    const method = bridge.pendingMethods.get(id) || 'unknown';
    bridge.pendingMethods.delete(id);
    bridge.sinks.traffic.addEntry({ direction, type: 'response', method, requestId: id, payload: message, error: 'error' in parsed });
    return;
  }
  if (isJsonRpcRequest(parsed)) {
    const id = parsed.id as JsonRpcId;
    if (direction === 'out') bridge.pendingMethods.set(id, parsed.method);
    bridge.sinks.traffic.addEntry({ direction, type: 'request', method: parsed.method, requestId: id, payload: message });
    return;
  }
  if (isJsonRpcNotification(parsed)) {
    bridge.sinks.traffic.addEntry({ direction, type: 'notification', method: parsed.method, payload: message });
  }
}
export function sendExplicitRelayCloseImpl(bridge: TransportBridge, socket: WebSocket | null): void {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  const payload = {
    jsonrpc: '2.0',
    id: nextRequestId(bridge.pendingRequests, bridge.timeoutHandles) as JsonRpcId,
    method: AGENT_METHODS.session_close,
    params: withAcpMeta(bridge.activeRelaySessionId ? { sessionId: bridge.activeRelaySessionId } : {}),
  };
  try {
    recordTrafficImpl(bridge, 'out', payload as AnyMessage);
    socket.send(JSON.stringify(payload));
  } catch {
    bridge.pendingMethods.delete(payload.id);
  }
}
export function withTimeoutImpl<T>(bridge: TransportBridge, method: string, timeoutMs: number, request: Promise<T>): Promise<T> {
  const requestId = nextRequestId(bridge.pendingRequests, bridge.timeoutHandles);
  return new Promise((resolve, reject) => {
    const handle = setTimeout(() => {
      bridge.timeoutHandles.delete(requestId);
      bridge.pendingRequests.delete(requestId);
      reject(new Error(`Request timeout: ${method}`));
    }, timeoutMs);
    bridge.timeoutHandles.set(requestId, handle);
    bridge.pendingRequests.set(requestId, { reject });
    request.then(
      (value) => {
        bridge.pendingRequests.delete(requestId);
        bridge.timeoutHandles.delete(requestId);
        clearTimeout(handle);
        resolve(value);
      },
      (error) => {
        bridge.pendingRequests.delete(requestId);
        bridge.timeoutHandles.delete(requestId);
        clearTimeout(handle);
        reject(error);
      }
    );
  });
}
export function rejectPendingRequestsImpl(bridge: TransportBridge, error: Error): void {
  for (const [requestId, pending] of bridge.pendingRequests) {
    const handle = bridge.timeoutHandles.get(requestId);
    if (handle) {
      clearTimeout(handle);
      bridge.timeoutHandles.delete(requestId);
    }
    pending.reject(error);
  }
  bridge.pendingRequests.clear();
}
function nextRequestId(pendingRequests: Map<number, TimeoutRequest>, timeoutHandles: Map<number, ReturnType<typeof setTimeout>>): number {
  let id = 0;
  while (pendingRequests.has(id) || timeoutHandles.has(id)) id += 1;
  return id;
}
