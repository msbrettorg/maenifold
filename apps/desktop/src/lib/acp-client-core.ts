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
import { killAgent, onAgentClosed, onAgentMessage, sendToAgent } from './tauri';

let trafficStore: ReturnType<typeof useTrafficStore> | null = null;

function getTrafficStore() {
  if (!trafficStore) {
    trafficStore = useTrafficStore();
  }
  return trafficStore;
}

export class AcpClientBridgeCore {
  protected agentId: string;
  protected permissionHandler: PermissionHandler;
  private messageResolvers: Map<JsonRpcId, (response: unknown) => void> = new Map();
  private messageRejecters: Map<JsonRpcId, (error: Error) => void> = new Map();
  private pendingMethods: Map<JsonRpcId, string> = new Map();
  private timeoutHandles: Map<JsonRpcId, ReturnType<typeof setTimeout>> = new Map();
  private nextRequestId = 0;
  private unlistenMessage: (() => void) | null = null;
  private unlistenClosed: (() => void) | null = null;
  protected supportedAgentMethods = new Set<string>();
  public pendingPermissionRequests: PermissionHandler['pendingPermissionRequests'];
  public onSessionUpdate: ((notification: SessionNotification) => void) | null = null;

  constructor(agentId: string, permissionHandler: PermissionHandler) {
    this.agentId = agentId;
    this.permissionHandler = permissionHandler;
    this.pendingPermissionRequests = permissionHandler.pendingPermissionRequests;
  }

  async connect(): Promise<void> {
    this.unlistenMessage = (await onAgentMessage((msg) => {
      if (msg.agent_id === this.agentId) {
        this.handleMessage(msg.message);
      }
    })) as unknown as () => void;
    this.unlistenClosed = (await onAgentClosed((closedAgentId) => {
      if (closedAgentId === this.agentId) {
        this.rejectPendingRequests(new Error(`ACP agent terminated before responding (${this.agentId})`));
      }
    })) as unknown as () => void;
  }

  async disconnect(): Promise<void> {
    if (this.unlistenMessage) {
      this.unlistenMessage();
      this.unlistenMessage = null;
    }
    if (this.unlistenClosed) {
      this.unlistenClosed();
      this.unlistenClosed = null;
    }
    this.supportedAgentMethods.clear();
    this.cancelPermission();
    this.rejectPendingRequests(new Error(`ACP client disconnected before response (${this.agentId})`));
    await killAgent(this.agentId);
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
      sendToAgent(this.agentId, JSON.stringify(request)).catch((e) => {
        this.clearRequest(id);
        reject(e);
      });
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

    await sendToAgent(this.agentId, JSON.stringify(notification));
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
        this.handleRequest(parsed.id, parsed.method, parsed.params);
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
    await sendToAgent(this.agentId, JSON.stringify(response));
  }

  private handleNotification(method: string, params: unknown): void {
    if (method === 'session/update' && this.onSessionUpdate) {
      this.onSessionUpdate(params as SessionNotification);
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
