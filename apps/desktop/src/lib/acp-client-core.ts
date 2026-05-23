import {
  AGENT_METHODS,
  ClientSideConnection,
  type CancelNotification,
  type Client,
  type CreateTerminalRequest,
  type CreateTerminalResponse,
  type KillTerminalRequest,
  type KillTerminalResponse,
  type ReadTextFileRequest,
  type ReadTextFileResponse,
  type ReleaseTerminalRequest,
  type ReleaseTerminalResponse,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionNotification,
  type Stream,
  type TerminalOutputRequest,
  type TerminalOutputResponse,
  type WaitForTerminalExitRequest,
  type WaitForTerminalExitResponse,
  type WriteTextFileRequest,
  type WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import { handleReadTextFileRequest, handleWriteTextFileRequest, type WritePermissionPolicy } from './acp-file-system';
import {
  createRejectAllWritesPolicy,
  createTerminalImpl,
  createWebSocketStreamImpl,
  killTerminalImpl,
  rejectPendingRequestsImpl,
  releaseTerminalImpl,
  sendExplicitRelayCloseImpl,
  terminalCreateImpl,
  terminalKillImpl,
  terminalOutputImpl,
  terminalReleaseImpl,
  terminalWaitForExitImpl,
  withTimeoutImpl,
  waitForTerminalExitImpl,
} from './acp-client-terminal';
import { withAcpMeta } from './acp-meta';
import { PermissionHandler } from './acp-permissions';
import { DAEMON_BASE_URL } from './daemon-url';
import type { AcpClientSinks } from './acp-sinks';
import { type JsonRpcId } from './acp-rpc';

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
    if (this.socket && this.socket.readyState === WebSocket.OPEN && this.connection) return;
    this.transportClosed = false;
    this.transportCloseReason = null;
    this.pendingMethods.clear();
    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(buildRelayUrl(this.agentName));
      this.socket = socket;
      this.connection = new ClientSideConnection(() => this, this.createWebSocketStream(socket));
      socket.addEventListener('open', () => resolve(), { once: true });
      socket.addEventListener('error', () => reject(new Error(`Failed to connect ACP relay for ${this.agentName}`)), {
        once: true,
      });
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
    if (socket && socket.readyState < WebSocket.CLOSING) socket.close(1000, 'client disconnect');
  }

  protected rememberActiveRelaySession(sessionId: string | undefined): void {
    if (typeof sessionId === 'string' && sessionId.trim()) this.activeRelaySessionId = sessionId;
  }

  protected clearActiveRelaySession(sessionId?: string): void {
    if (!sessionId || this.activeRelaySessionId === sessionId) this.activeRelaySessionId = null;
  }

  private terminalRequestBridge() {
    return { sendRequest: <T>(method: string, requestParams?: unknown) => this.sendRequest<T>(method, requestParams), sinks: this.sinks };
  }

  private transportBridge() {
    return {
      agentName: this.agentName,
      pendingMethods: this.pendingMethods,
      timeoutHandles: this.timeoutHandles,
      pendingRequests: this.pendingRequests,
      activeRelaySessionId: this.activeRelaySessionId,
      sinks: this.sinks,
      getOnSessionUpdate: () => this.onSessionUpdate,
      extNotification: (method: string, params: Record<string, unknown>) => this.extNotification(method, params),
    };
  }

  supportsAgentMethod(method: string): boolean {
    return this.supportedAgentMethods.has(method);
  }

  protected async sendRequest<T>(method: string, params?: unknown): Promise<T> {
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
    return createTerminalImpl({ sinks: this.sinks }, params);
  }

  async terminalOutput(params: TerminalOutputRequest): Promise<TerminalOutputResponse> {
    return terminalOutputImpl(this.terminalRequestBridge(), params);
  }

  async waitForTerminalExit(params: WaitForTerminalExitRequest): Promise<WaitForTerminalExitResponse> {
    return waitForTerminalExitImpl({ sinks: this.sinks }, params);
  }

  async killTerminal(params: KillTerminalRequest): Promise<KillTerminalResponse> {
    return killTerminalImpl(params);
  }

  async releaseTerminal(params: ReleaseTerminalRequest): Promise<ReleaseTerminalResponse> {
    return releaseTerminalImpl({ sinks: this.sinks }, params);
  }

  async terminalCreate(params: CreateTerminalRequest): Promise<CreateTerminalResponse> {
    return terminalCreateImpl(this.terminalRequestBridge(), params);
  }

  async terminalWaitForExit(params: WaitForTerminalExitRequest): Promise<WaitForTerminalExitResponse> {
    return terminalWaitForExitImpl(this.terminalRequestBridge(), params);
  }

  async terminalKill(params: KillTerminalRequest): Promise<KillTerminalResponse> {
    return terminalKillImpl(this.terminalRequestBridge(), params);
  }

  async terminalRelease(params: ReleaseTerminalRequest): Promise<ReleaseTerminalResponse> {
    return terminalReleaseImpl(this.terminalRequestBridge(), params);
  }

  async extNotification(method: string, params: Record<string, unknown>): Promise<void> {
    if (method !== '_relay/stderr') return;
    const line = typeof params.data === 'string' ? params.data : '';
    if (line) this.onStderr?.(line);
  }

  private requireConnection(): ClientSideConnection {
    if (!this.connection) throw new Error(`ACP relay socket is not connected (${this.agentName})`);
    return this.connection;
  }

  private createWebSocketStream(socket: WebSocket): Stream {
    return createWebSocketStreamImpl(this.transportBridge(), socket);
  }

  private sendExplicitRelayClose(socket: WebSocket | null): void {
    sendExplicitRelayCloseImpl(this.transportBridge(), socket);
  }

  private withTimeout<T>(method: string, request: Promise<T>): Promise<T> {
    return withTimeoutImpl(this.transportBridge(), method, this.getTimeoutForMethod(method), request);
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

  private rejectPendingRequests(error: Error): void {
    rejectPendingRequestsImpl(this.transportBridge(), error);
  }
}
