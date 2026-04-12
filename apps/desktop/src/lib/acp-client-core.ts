import type {
  CancelNotification,
  ReadTextFileRequest,
  ReadTextFileResponse,
  RequestPermissionRequest,
  RequestPermissionResponse,
  SessionNotification,
  WriteTextFileRequest,
  WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import { useTrafficStore } from '../stores/traffic';
import { handleReadTextFileRequest, handleWriteTextFileRequest } from './acp-file-system';
import { PermissionHandler } from './acp-permissions';
import { DAEMON_BASE_URL } from './daemon-url';
import {
  asJsonRpcRecord,
  createJsonRpcErrorResponse,
  createJsonRpcNotification,
  createJsonRpcRequest,
  createJsonRpcSuccessResponse,
  isJsonRpcNotification,
  isJsonRpcRequest,
  isJsonRpcResponse,
  type JsonRpcError,
  type JsonRpcId,
} from './acp-rpc';

let trafficStore: ReturnType<typeof useTrafficStore> | null = null;

function getTrafficStore() {
  if (!trafficStore) {
    trafficStore = useTrafficStore();
  }
  return trafficStore;
}

function buildRelayUrl(agentName: string): string {
  const base = new URL(DAEMON_BASE_URL);
  base.protocol = base.protocol === 'https:' ? 'wss:' : 'ws:';
  base.pathname = '/api/acp/session';
  base.search = '';
  base.searchParams.set('agent', agentName);
  base.searchParams.set('keepSessionAlive', 'true');
  return base.toString();
}

export class AcpClientBridgeCore {
  protected agentId: string;
  protected agentName: string;
  protected permissionHandler: PermissionHandler;
  private messageResolvers: Map<JsonRpcId, (response: unknown) => void> = new Map();
  private messageRejecters: Map<JsonRpcId, (error: Error) => void> = new Map();
  private pendingMethods: Map<JsonRpcId, string> = new Map();
  private timeoutHandles: Map<JsonRpcId, ReturnType<typeof setTimeout>> = new Map();
  private nextRequestId = 0;
  private socket: WebSocket | null = null;
  private transportClosed = false;
  private transportCloseReason: string | null = null;
  protected supportedAgentMethods = new Set<string>();
  public pendingPermissionRequests: PermissionHandler['pendingPermissionRequests'];
  public onSessionUpdate: ((notification: SessionNotification) => void) | null = null;
  public onStderr: ((line: string) => void) | null = null;

  constructor(agentId: string, agentName: string, permissionHandler: PermissionHandler) {
    this.agentId = agentId;
    this.agentName = agentName;
    this.permissionHandler = permissionHandler;
    this.pendingPermissionRequests = permissionHandler.pendingPermissionRequests;
  }

  async connect(): Promise<void> {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      return;
    }

    this.transportClosed = false;
    this.transportCloseReason = null;

    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(buildRelayUrl(this.agentName));
      this.socket = socket;

      socket.addEventListener('open', () => resolve(), { once: true });
      socket.addEventListener('error', () => {
        reject(new Error(`Failed to connect ACP relay for ${this.agentName}`));
      }, { once: true });
      socket.addEventListener('message', (event) => {
        if (typeof event.data === 'string') {
          this.handleMessage(event.data);
        }
      });
      socket.addEventListener('close', (event) => {
        this.transportClosed = true;
        this.transportCloseReason = event.reason || `socket closed (${event.code})`;
        this.socket = null;
        this.rejectPendingRequests(
          new Error(`ACP agent terminated before responding (${this.agentName})`),
        );
      });
    });
  }

  isTransportClosed(): boolean {
    return this.transportClosed;
  }

  getTransportCloseReason(): string | null {
    return this.transportCloseReason;
  }

  async disconnect(): Promise<void> {
    this.supportedAgentMethods.clear();
    this.cancelPermission();
    this.rejectPendingRequests(new Error(`ACP client disconnected before response (${this.agentName})`));

    const socket = this.socket;
    this.socket = null;
    if (socket && socket.readyState < WebSocket.CLOSING) {
      socket.close(1000, 'client disconnect');
    }
  }

  private clearRequest(id: JsonRpcId): void {
    const handle = this.timeoutHandles.get(id);
    if (handle !== undefined) {
      clearTimeout(handle);
      this.timeoutHandles.delete(id);
    }
    this.messageResolvers.delete(id);
    this.messageRejecters.delete(id);
    this.pendingMethods.delete(id);
  }

  private rejectPendingRequests(error: Error): void {
    const pendingRejecters = [...this.messageRejecters.entries()];
    for (const [id, rejecter] of pendingRejecters) {
      this.clearRequest(id);
      rejecter(error);
    }
  }

  supportsAgentMethod(method: string): boolean {
    return this.supportedAgentMethods.has(method);
  }

  protected async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    const id = this.nextRequestId++;
    const request = createJsonRpcRequest(id, method, params);
    const store = getTrafficStore();
    store.addEntry({
      direction: 'out',
      type: 'request',
      method,
      requestId: id,
      payload: request,
    });
    this.pendingMethods.set(id, method);

    if (import.meta.env.DEV) {
      console.log('Sending request:', request);
    }
    const timeoutMs = this.getTimeoutForMethod(method);

    return new Promise((resolve, reject) => {
      this.messageResolvers.set(id, (response) => {
        resolve(response as T);
      });
      this.messageRejecters.set(id, reject);
      try {
        this.sendRaw(JSON.stringify(request));
      } catch (e) {
        this.clearRequest(id);
        reject(e instanceof Error ? e : new Error(String(e)));
        return;
      }
      const handle = setTimeout(() => {
        if (this.messageResolvers.has(id)) {
          this.clearRequest(id);
          reject(new Error(`Request timeout: ${method}`));
        }
      }, timeoutMs);
      this.timeoutHandles.set(id, handle);
    });
  }

  protected async sendNotification(method: string, params?: unknown): Promise<void> {
    const notification = createJsonRpcNotification(method, params);
    const store = getTrafficStore();
    store.addEntry({
      direction: 'out',
      type: 'notification',
      method,
      payload: notification,
    });

    this.sendRaw(JSON.stringify(notification));
  }

  private sendRaw(payload: string): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      throw new Error(`ACP relay socket is not connected (${this.agentName})`);
    }

    this.socket.send(payload);
  }

  async cancel(params: CancelNotification): Promise<void> {
    await this.sendNotification('session/cancel', params);
  }

  async requestPermission(
    params: RequestPermissionRequest
  ): Promise<RequestPermissionResponse> {
    return this.permissionHandler.requestPermission(params);
  }

  resolvePermission(requestId: string, optionId: string): void {
    this.permissionHandler.resolvePermission(requestId, optionId);
  }

  cancelPermission(): void {
    for (const request of this.permissionHandler.getPermissions()) {
      this.permissionHandler.cancelPermission(request.toolCall.toolCallId);
    }
  }

  async sessionUpdate(_params: SessionNotification): Promise<void> {}

  async writeTextFile(params: WriteTextFileRequest): Promise<WriteTextFileResponse> {
    return handleWriteTextFileRequest(params);
  }

  async readTextFile(params: ReadTextFileRequest): Promise<ReadTextFileResponse> {
    return handleReadTextFileRequest(params);
  }

  private handleMessage(message: string): void {
    try {
      const parsed = asJsonRpcRecord(JSON.parse(message));
      const store = getTrafficStore();

      if (isJsonRpcResponse(parsed)) {
        const method = this.pendingMethods.get(parsed.id) || 'unknown';
        const resolver = this.messageResolvers.get(parsed.id);
        const rejecter = this.messageRejecters.get(parsed.id);
        this.clearRequest(parsed.id);
        store.addEntry({
          direction: 'in',
          type: 'response',
          method,
          requestId: parsed.id,
          payload: parsed,
          error: !!parsed.error,
        });
        if (resolver && rejecter) {
          if (parsed.error) {
            console.error('JSON-RPC error:', parsed.error);
            rejecter(new Error(parsed.error.message || 'Unknown error'));
          } else {
            resolver(parsed.result);
          }
        }
      }

      if (isJsonRpcRequest(parsed)) {
        store.addEntry({
          direction: 'in',
          type: 'request',
          method: parsed.method,
          requestId: parsed.id,
          payload: parsed,
        });
        void this.handleRequest(parsed.id, parsed.method, parsed.params);
      }

      if (isJsonRpcNotification(parsed)) {
        store.addEntry({
          direction: 'in',
          type: 'notification',
          method: parsed.method,
          payload: parsed,
        });
        this.handleNotification(parsed.method, parsed.params);
      }
    } catch (e) {
      console.error('Failed to parse message:', message, e);
    }
  }

  private async handleRequest(id: JsonRpcId, method: string, params: unknown): Promise<void> {
    let result: unknown;
    let error: JsonRpcError | undefined;

    try {
      switch (method) {
        case 'fs/read_text_file':
          result = await this.readTextFile(params as ReadTextFileRequest);
          break;
        case 'fs/write_text_file':
          result = await this.writeTextFile(params as WriteTextFileRequest);
          break;
        case 'session/request_permission':
          result = await this.requestPermission(params as RequestPermissionRequest);
          break;
        default:
          error = { code: -32601, message: `Method not found: ${method}` };
      }
    } catch (e) {
      const code =
        typeof (e as { code?: unknown })?.code === 'number' ? (e as { code: number }).code : -32603;
      error = { code, message: e instanceof Error ? e.message : String(e) };
    }

    const response = error
      ? createJsonRpcErrorResponse(id, error)
      : createJsonRpcSuccessResponse(id, result);
    const store = getTrafficStore();
    store.addEntry({
      direction: 'out',
      type: 'response',
      method,
      requestId: id,
      payload: response,
      error: !!error,
    });
    this.sendRaw(JSON.stringify(response));
  }

  private handleNotification(method: string, params: unknown): void {
    if (method === 'session/update' && this.onSessionUpdate) {
      this.onSessionUpdate(params as SessionNotification);
      return;
    }

    if (method === '_relay/stderr' && this.onStderr) {
      const line = typeof (params as { data?: unknown })?.data === 'string'
        ? (params as { data: string }).data
        : '';
      if (line) {
        this.onStderr(line);
      }
    }
  }

  private getTimeoutForMethod(method: string): number {
    switch (method) {
      case 'session/prompt':
      case 'terminal/wait_for_exit':
      case 'session/set_config_option':
        return 30 * 60 * 1000;
      case 'terminal/create':
      case 'session/load':
        return 5 * 60 * 1000;
      default:
        return 60 * 1000;
    }
  }
}
