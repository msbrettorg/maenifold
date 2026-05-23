import {
  AGENT_METHODS,
  ClientSideConnection,
  type AnyMessage,
  type CancelNotification,
  type Client,
  type ReadTextFileRequest,
  type ReadTextFileResponse,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionNotification,
  type Stream,
  type CreateTerminalRequest,
  type CreateTerminalResponse,
  type TerminalOutputRequest,
  type TerminalOutputResponse,
  type WaitForTerminalExitRequest,
  type WaitForTerminalExitResponse,
  type KillTerminalRequest,
  type KillTerminalResponse,
  type ReleaseTerminalRequest,
  type ReleaseTerminalResponse,
  type WriteTextFileRequest,
  type WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import {
  handleReadTextFileRequest,
  handleWriteTextFileRequest,
  type WritePermissionPolicy,
} from './acp-file-system';
import { withAcpMeta } from './acp-meta';
import { PermissionHandler } from './acp-permissions';
import { DAEMON_BASE_URL } from './daemon-url';
import type { AcpClientSinks } from './acp-sinks';
import {
  asJsonRpcRecord,
  isJsonRpcNotification,
  isJsonRpcRequest,
  isJsonRpcResponse,
  type JsonRpcId,
} from './acp-rpc';

type CreateTerminalResponseWithLimit = CreateTerminalResponse & {
  outputByteLimit?: number;
};

function createRejectAllWritesPolicy(): WritePermissionPolicy {
  return {
    currentModeId: () => null,
    notifyPlanModeBlocked: () => {},
  };
}

function requireNonBlank(value: string, fieldName: string): void {
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`terminal request requires ${fieldName}`);
  }
}

function validateExitStatus(response: WaitForTerminalExitResponse): WaitForTerminalExitResponse {
  if (
    (response.exitCode != null && typeof response.exitCode !== 'number') ||
    (response.signal != null && typeof response.signal !== 'string')
  ) {
    throw new Error('Invalid terminal exit response');
  }
  return {
    exitCode: response.exitCode ?? null,
    signal: response.signal ?? null,
  };
}

function validateTerminalOutput(response: TerminalOutputResponse): TerminalOutputResponse {
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

function buildRelayUrl(agentName: string): string {
  const base = new URL(DAEMON_BASE_URL);
  base.protocol = base.protocol === 'https:' ? 'wss:' : 'ws:';
  base.pathname = '/api/acp/session';
  base.search = '';
  base.searchParams.set('agent', agentName);
  base.searchParams.set('keepSessionAlive', 'true');
  return base.toString();
}

export class AcpClientBridgeCore implements Client {
  protected agentId: string;
  protected agentName: string;
  protected permissionHandler: PermissionHandler;
  private connection: ClientSideConnection | null = null;
  private pendingMethods: Map<JsonRpcId, string> = new Map();
  private timeoutHandles: Map<number, ReturnType<typeof setTimeout>> = new Map();
  private pendingRequests: Map<number, { reject: (error: Error) => void }> = new Map();
  private socket: WebSocket | null = null;
  protected activeRelaySessionId: string | null = null;
  private transportClosed = false;
  private transportCloseReason: string | null = null;
  protected supportedAgentMethods = new Set<string>();
  public pendingPermissionRequests: PermissionHandler['pendingPermissionRequests'];
  public onSessionUpdate: ((notification: SessionNotification) => void) | null = null;
  public onStderr: ((line: string) => void) | null = null;

  constructor(
    agentId: string,
    agentName: string,
    permissionHandler: PermissionHandler,
    private readonly sinks: AcpClientSinks,
    private readonly writePolicy: WritePermissionPolicy = createRejectAllWritesPolicy()
  ) {
    this.agentId = agentId;
    this.agentName = agentName;
    this.permissionHandler = permissionHandler;
    this.pendingPermissionRequests = permissionHandler.pendingPermissionRequests;
  }

  setPermissionModeProvider(currentModeId: () => string | null): void {
    this.permissionHandler.setCurrentModeProvider(currentModeId);
  }

  async connect(): Promise<void> {
    if (this.socket && this.socket.readyState === WebSocket.OPEN && this.connection) {
      return;
    }

    this.transportClosed = false;
    this.transportCloseReason = null;
    this.pendingMethods.clear();

    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(buildRelayUrl(this.agentName));
      this.socket = socket;
      this.connection = new ClientSideConnection(() => this, this.createWebSocketStream(socket));

      socket.addEventListener('open', () => resolve(), { once: true });
      socket.addEventListener(
        'error',
        () => reject(new Error(`Failed to connect ACP relay for ${this.agentName}`)),
        { once: true }
      );
      socket.addEventListener('close', (event) => {
        this.transportClosed = true;
        this.transportCloseReason = event.reason || `socket closed (${event.code})`;
        this.socket = null;
        this.rejectPendingRequests(new Error(`ACP agent terminated before responding (${this.agentName})`));
        this.pendingMethods.clear();
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
    const socket = this.socket;
    this.sendExplicitRelayClose(socket);
    this.supportedAgentMethods.clear();
    this.cancelPermission();
    this.rejectPendingRequests(new Error(`ACP client disconnected before response (${this.agentName})`));
    this.pendingMethods.clear();
    this.activeRelaySessionId = null;

    this.socket = null;
    this.connection = null;
    if (socket && socket.readyState < WebSocket.CLOSING) {
      socket.close(1000, 'client disconnect');
    }
  }

  protected rememberActiveRelaySession(sessionId: string | undefined): void {
    if (typeof sessionId === 'string' && sessionId.trim()) {
      this.activeRelaySessionId = sessionId;
    }
  }

  protected clearActiveRelaySession(sessionId?: string): void {
    if (!sessionId || this.activeRelaySessionId === sessionId) {
      this.activeRelaySessionId = null;
    }
  }

  supportsAgentMethod(method: string): boolean {
    return this.supportedAgentMethods.has(method);
  }

  protected async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    // All ACP relay requests flow through sendRequest, which combines:
    // 1. SDK ClientSideConnection.extMethod for request/response correlation.
    // 2. Bridge-level withTimeout for per-method timeout enforcement.
    // There is intentionally no redundant timeout in the SDK config or in a
    // terminal-specific request path; withTimeout is the single timeout layer.
    const connection = this.requireConnection();
    const request = connection.extMethod(method, withAcpMeta(params ?? {}));
    return this.withTimeout(method, request as Promise<T>);
  }

  protected async sendNotification(method: string, params?: unknown): Promise<void> {
    await this.requireConnection().extNotification(method, withAcpMeta(params ?? {}));
  }

  async cancel(params: CancelNotification): Promise<void> {
    await this.sendNotification(AGENT_METHODS.session_cancel, params);
  }

  async requestPermission(params: RequestPermissionRequest): Promise<RequestPermissionResponse> {
    // PermissionHandler performs mode supersede and persisted-ledger lookup before
    // exposing anything through pendingPermissionRequests for the lifted dialog.
    return this.permissionHandler.requestPermission(params);
  }

  async resolvePermission(
    requestId: string,
    optionId: string,
    scope?: 'session' | 'agent' | 'forever'
  ): Promise<void> {
    try {
      await this.permissionHandler.resolvePermission(requestId, optionId, scope);
    } catch (error) {
      console.error('Failed to resolve permission decision:', error);
    }
  }

  cancelPermission(): void {
    for (const request of this.permissionHandler.getPermissions()) {
      this.permissionHandler.cancelPermission(request.toolCall.toolCallId);
    }
  }

  async sessionUpdate(params: SessionNotification): Promise<void> {
    this.onSessionUpdate?.(params);
  }

  async writeTextFile(params: WriteTextFileRequest): Promise<WriteTextFileResponse> {
    return handleWriteTextFileRequest(params, this.writePolicy);
  }

  async readTextFile(params: ReadTextFileRequest): Promise<ReadTextFileResponse> {
    return handleReadTextFileRequest(params);
  }

  async createTerminal(params: CreateTerminalRequest): Promise<CreateTerminalResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.command, 'command');
    const api = window.electronAPI?.terminal?.create;
    if (!api) throw new Error('Terminal API is unavailable');
    const response = await api(params);
    this.sinks.terminals.setTerminal(response.terminalId, {
      terminalId: response.terminalId,
      output: '',
      truncated: false,
      released: false,
      exitStatus: null,
      outputByteLimit: response.outputByteLimit,
    });
    return response;
  }

  async terminalOutput(params: TerminalOutputRequest): Promise<TerminalOutputResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    const response = validateTerminalOutput(
      await this.sendRequest<TerminalOutputResponse>('terminal/output', params)
    );
    this.sinks.terminals.setTerminal(params.terminalId, {
      terminalId: params.terminalId,
      output: response.output,
      truncated: response.truncated,
      released: false,
      exitStatus: response.exitStatus ?? null,
    });
    return response;
  }

  async waitForTerminalExit(params: WaitForTerminalExitRequest): Promise<WaitForTerminalExitResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    const api = window.electronAPI?.terminal?.waitForExit;
    if (!api) throw new Error('Terminal API is unavailable');
    const response = validateExitStatus(await api(params));
    this.sinks.terminals.markExited(params.terminalId, response);
    return response;
  }

  async killTerminal(params: KillTerminalRequest): Promise<KillTerminalResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    const api = window.electronAPI?.terminal?.kill;
    if (!api) throw new Error('Terminal API is unavailable');
    return api(params);
  }

  async releaseTerminal(params: ReleaseTerminalRequest): Promise<ReleaseTerminalResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    const api = window.electronAPI?.terminal?.release;
    if (!api) throw new Error('Terminal API is unavailable');
    const response = await api(params);
    this.sinks.terminals.markReleased(params.terminalId);
    return response;
  }

  async terminalCreate(params: CreateTerminalRequest): Promise<CreateTerminalResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.command, 'command');
    const response = await this.sendRequest<CreateTerminalResponseWithLimit>('terminal/create', params);
    this.sinks.terminals.setTerminal(response.terminalId, {
      terminalId: response.terminalId,
      output: '',
      truncated: false,
      released: false,
      exitStatus: null,
      outputByteLimit: response.outputByteLimit,
    });
    return response;
  }

  async terminalWaitForExit(params: WaitForTerminalExitRequest): Promise<WaitForTerminalExitResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    const response = validateExitStatus(
      await this.sendRequest<WaitForTerminalExitResponse>('terminal/wait_for_exit', params)
    );
    this.sinks.terminals.markExited(params.terminalId, response);
    return response;
  }

  async terminalKill(params: KillTerminalRequest): Promise<KillTerminalResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    return this.sendRequest<KillTerminalResponse>('terminal/kill', params);
  }

  async terminalRelease(params: ReleaseTerminalRequest): Promise<ReleaseTerminalResponse> {
    requireNonBlank(params.sessionId, 'sessionId');
    requireNonBlank(params.terminalId, 'terminalId');
    const response = await this.sendRequest<ReleaseTerminalResponse>('terminal/release', params);
    this.sinks.terminals.markReleased(params.terminalId);
    return response;
  }

  async extNotification(method: string, params: Record<string, unknown>): Promise<void> {
    if (method !== '_relay/stderr') {
      return;
    }

    const line = typeof params.data === 'string' ? params.data : '';
    if (line) {
      this.onStderr?.(line);
    }
  }

  private requireConnection(): ClientSideConnection {
    if (!this.connection) {
      throw new Error(`ACP relay socket is not connected (${this.agentName})`);
    }
    return this.connection;
  }

  // SDK parity gap: no browser WebSocket relay Stream adapter with traffic-store hooks.
  private createWebSocketStream(socket: WebSocket): Stream {
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
              if (this.handleAcpUiNotification(message)) continue;
              this.recordTraffic('in', message);
              controller.enqueue(message);
            } catch (error) {
              console.error('Failed to handle ACP relay message:', error);
            }
          }
        });
        socket.addEventListener('close', () => closeReadable(controller));
        socket.addEventListener('error', () => {
          controller.error(new Error(`ACP relay socket errored (${this.agentName})`));
        });
      },
      cancel: () => {
        if (socket.readyState < WebSocket.CLOSING) {
          socket.close(1000, 'stream cancelled');
        }
      },
    });

    const writable = new WritableStream<AnyMessage>({
      write: (message) => {
        if (socket.readyState !== WebSocket.OPEN) {
          throw new Error(`ACP relay socket is not connected (${this.agentName})`);
        }
        this.recordTraffic('out', message);
        socket.send(JSON.stringify(message));
      },
    });

    return { readable, writable };
  }

  private handleAcpUiNotification(message: AnyMessage): boolean {
    const parsed = asJsonRpcRecord(message);
    if (!isJsonRpcNotification(parsed)) {
      return false;
    }

    if (parsed.method === 'session/update') {
      this.recordTraffic('in', message);
      try {
        this.onSessionUpdate?.((parsed.params ?? {}) as SessionNotification);
      } catch (error) {
        console.error('Failed to handle ACP session/update notification:', error);
      }
      return true;
    }

    if (parsed.method === '_relay/stderr') {
      this.recordTraffic('in', message);
      const params = asJsonRpcRecord(parsed.params);
      void this.extNotification(parsed.method, params);
      return true;
    }

    return false;
  }

  private recordTraffic(direction: 'in' | 'out', message: AnyMessage): void {
    const parsed = asJsonRpcRecord(message);

    if (isJsonRpcResponse(parsed)) {
      const id = parsed.id as JsonRpcId;
      const method = this.pendingMethods.get(id) || 'unknown';
      this.pendingMethods.delete(id);
      this.sinks.traffic.addEntry({
        direction,
        type: 'response',
        method,
        requestId: id,
        payload: message,
        error: 'error' in parsed,
      });
      return;
    }

    if (isJsonRpcRequest(parsed)) {
      const id = parsed.id as JsonRpcId;
      if (direction === 'out') {
        this.pendingMethods.set(id, parsed.method);
      }
      this.sinks.traffic.addEntry({
        direction,
        type: 'request',
        method: parsed.method,
        requestId: id,
        payload: message,
      });
      return;
    }

    if (isJsonRpcNotification(parsed)) {
      this.sinks.traffic.addEntry({
        direction,
        type: 'notification',
        method: parsed.method,
        payload: message,
      });
    }
  }

  private sendExplicitRelayClose(socket: WebSocket | null): void {
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return;
    }
    const payload = {
      jsonrpc: '2.0',
      id: this.nextRequestId() as JsonRpcId,
      method: AGENT_METHODS.session_close,
      params: withAcpMeta(this.activeRelaySessionId ? { sessionId: this.activeRelaySessionId } : {}),
    };
    try {
      this.recordTraffic('out', payload as AnyMessage);
      socket.send(JSON.stringify(payload));
    } catch {
      this.pendingMethods.delete(payload.id);
    }
  }

  private withTimeout<T>(method: string, request: Promise<T>): Promise<T> {
    // SDK parity gap: ClientSideConnection owns pending responses but does not expose
    // per-method timeout configuration or semantic close errors, so the bridge retains
    // the UI-level timeout policy and rejects in-flight requests on transport shutdown.
    const requestId = this.nextRequestId();
    return new Promise((resolve, reject) => {
      const handle = setTimeout(
        () => {
          this.timeoutHandles.delete(requestId);
          this.pendingRequests.delete(requestId);
          reject(new Error(`Request timeout: ${method}`));
        },
        this.getTimeoutForMethod(method)
      );
      this.timeoutHandles.set(requestId, handle);
      this.pendingRequests.set(requestId, { reject });
      request.then(
        (value) => {
          this.pendingRequests.delete(requestId);
          this.timeoutHandles.delete(requestId);
          clearTimeout(handle);
          resolve(value);
        },
        (error) => {
          this.pendingRequests.delete(requestId);
          this.timeoutHandles.delete(requestId);
          clearTimeout(handle);
          reject(error);
        }
      );
    });
  }

  private nextRequestId(): number {
    let id = 0;
    while (this.pendingRequests.has(id) || this.timeoutHandles.has(id)) {
      id += 1;
    }
    return id;
  }

  private rejectPendingRequests(error: Error): void {
    for (const [requestId, pending] of this.pendingRequests) {
      const handle = this.timeoutHandles.get(requestId);
      if (handle) {
        clearTimeout(handle);
        this.timeoutHandles.delete(requestId);
      }
      pending.reject(error);
    }
    this.pendingRequests.clear();
  }

  private getTimeoutForMethod(method: string): number {
    switch (method) {
      case AGENT_METHODS.session_prompt:
      case 'terminal/wait_for_exit':
      case AGENT_METHODS.session_set_config_option:
        return 30 * 60 * 1000;
      case 'terminal/create':
      case AGENT_METHODS.session_load:
        return 5 * 60 * 1000;
      default:
        return 60 * 1000;
    }
  }
}
