// ACP Client Bridge - Adapts Tauri IPC to ACP SDK
import type {
  Client,
  SessionNotification,
  RequestPermissionRequest,
  RequestPermissionResponse,
  WriteTextFileRequest,
  WriteTextFileResponse,
  ReadTextFileRequest,
  ReadTextFileResponse,
  InitializeRequest,
  InitializeResponse,
  NewSessionRequest,
  NewSessionResponse,
  LoadSessionRequest,
  LoadSessionResponse,
  PromptRequest,
  PromptResponse,
  CancelNotification,
  AuthenticateRequest,
  AuthenticateResponse,
} from '@agentclientprotocol/sdk';
import {
  mkdir,
  readTextFile as tauriReadTextFile,
  writeTextFile as tauriWriteTextFile,
} from '@tauri-apps/plugin-fs';
import { dirname } from '@tauri-apps/api/path';
import { sendToAgent, onAgentMessage, killAgent } from './tauri';
import type {
  AcpMetadata,
  AgentInstance,
  PermissionRequest as LocalPermissionRequest,
} from './types';
import { ref, type Ref } from 'vue';
import { useTrafficStore } from '../stores/traffic';

// Event emitter for permission requests
type PermissionResolver = (response: RequestPermissionResponse) => void;

// Traffic store instance (lazily initialized)
let trafficStore: ReturnType<typeof useTrafficStore> | null = null;
function getTrafficStore() {
  if (!trafficStore) {
    trafficStore = useTrafficStore();
  }
  return trafficStore;
}

const WINDOWS_ABSOLUTE_PATH = /^[A-Za-z]:[\\/]/;
const ACP_EXTENSION_META_KEY = 'maenifold/extensions';

export const SESSION_SET_MODEL_METHOD = '_session/set_model';
const SESSION_SET_CONFIG_OPTION_METHOD = 'session/set_config_option';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function cloneMeta(meta: unknown): Record<string, unknown> | null {
  return isRecord(meta) ? { ...meta } : null;
}

function withAcpMeta<T>(
  value: T,
  extraMeta?: Record<string, unknown>
): (T extends Record<string, unknown> ? T : Record<string, unknown>) & {
  _meta: AcpMetadata;
} {
  const base: Record<string, unknown> = isRecord(value) ? { ...value } : {};
  const existingMeta = cloneMeta(base._meta);
  const mergedMeta =
    existingMeta || extraMeta ? { ...(existingMeta ?? {}), ...(extraMeta ?? {}) } : null;
  return {
    ...base,
    _meta: mergedMeta,
  } as (T extends Record<string, unknown> ? T : Record<string, unknown>) & {
    _meta: AcpMetadata;
  };
}

function readAdvertisedAgentMethods(meta: unknown): Set<string> {
  if (!isRecord(meta)) {
    return new Set();
  }

  const extensions = meta[ACP_EXTENSION_META_KEY];
  if (!isRecord(extensions) || !Array.isArray(extensions.supportedAgentMethods)) {
    return new Set();
  }

  return new Set(
    extensions.supportedAgentMethods.filter(
      (method): method is string => typeof method === 'string'
    )
  );
}

function withExtensionCapabilities(
  capabilities: unknown
): Record<string, unknown> & { _meta: AcpMetadata } {
  const existing = isRecord(capabilities) ? { ...capabilities } : {};
  const existingMeta = cloneMeta(existing._meta);
  const existingExtensions = isRecord(existingMeta?.[ACP_EXTENSION_META_KEY])
    ? { ...(existingMeta?.[ACP_EXTENSION_META_KEY] as Record<string, unknown>) }
    : {};
  const supportedAgentMethods = Array.isArray(existingExtensions.supportedAgentMethods)
    ? existingExtensions.supportedAgentMethods.filter(
        (method): method is string => typeof method === 'string'
      )
    : [];

  if (!supportedAgentMethods.includes(SESSION_SET_MODEL_METHOD)) {
    supportedAgentMethods.push(SESSION_SET_MODEL_METHOD);
  }

  return {
    ...existing,
    _meta: {
      ...(existingMeta ?? {}),
      [ACP_EXTENSION_META_KEY]: {
        ...existingExtensions,
        supportedAgentMethods,
      },
    },
  };
}

function assertAbsolutePath(path: string): void {
  if (!path.startsWith('/') && !WINDOWS_ABSOLUTE_PATH.test(path)) {
    throw Object.assign(new Error('Path must be absolute'), { code: -32602 });
  }
}

export class AcpClientBridge implements Client {
  private agentId: string;
  private messageResolvers: Map<number, (response: unknown) => void> = new Map();
  private messageRejecters: Map<number, (error: Error) => void> = new Map();
  private pendingMethods: Map<number, string> = new Map(); // Track method names for responses
  private nextRequestId = 0;
  private unlistenMessage: (() => void) | null = null;
  private supportedAgentMethods = new Set<string>();
  
  // Permission request handling
  public pendingPermissionRequest: Ref<LocalPermissionRequest | null> = ref(null);
  private permissionResolver: PermissionResolver | null = null;

  // Session update callback
  public onSessionUpdate: ((notification: SessionNotification) => void) | null = null;

  constructor(agentId: string) {
    this.agentId = agentId;
  }

  async connect(): Promise<void> {
    this.unlistenMessage = await onAgentMessage((msg) => {
      if (msg.agent_id === this.agentId) {
        this.handleMessage(msg.message);
      }
    }) as unknown as () => void;
  }

  async disconnect(): Promise<void> {
    if (this.unlistenMessage) {
      this.unlistenMessage();
      this.unlistenMessage = null;
    }
    this.supportedAgentMethods.clear();
    await killAgent(this.agentId);
  }

  supportsAgentMethod(method: string): boolean {
    return this.supportedAgentMethods.has(method);
  }

  private handleMessage(message: string): void {
    try {
      const parsed = JSON.parse(message);
      const store = getTrafficStore();
      
      // Handle JSON-RPC response (has id and result/error, no method)
      if ('id' in parsed && parsed.id !== undefined && !('method' in parsed)) {
        // Track incoming response
        store.addEntry({
          direction: 'in',
          type: 'response',
          method: this.pendingMethods.get(parsed.id) || 'unknown',
          requestId: parsed.id,
          payload: parsed,
          error: !!parsed.error,
        });
        this.pendingMethods.delete(parsed.id);
        
        const resolver = this.messageResolvers.get(parsed.id);
        const rejecter = this.messageRejecters.get(parsed.id);
        if (resolver && rejecter) {
          this.messageResolvers.delete(parsed.id);
          this.messageRejecters.delete(parsed.id);
          if (parsed.error) {
            console.error('JSON-RPC error:', parsed.error);
            rejecter(new Error(parsed.error.message || 'Unknown error'));
          } else {
            resolver(parsed.result);
          }
        }
      }
      
      // Handle JSON-RPC request from agent (has id and method)
      if ('id' in parsed && parsed.id !== undefined && 'method' in parsed) {
        // Track incoming request from agent
        store.addEntry({
          direction: 'in',
          type: 'request',
          method: parsed.method,
          requestId: parsed.id,
          payload: parsed,
        });
        this.handleRequest(parsed.id, parsed.method, parsed.params);
      }
      
      // Handle JSON-RPC notification (no id, has method)
      if (!('id' in parsed) && parsed.method) {
        // Track incoming notification
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

  private async handleRequest(id: number | string, method: string, params: unknown): Promise<void> {
    let result: unknown;
    let error: { code: number; message: string } | undefined;

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
        typeof (e as { code?: unknown })?.code === 'number'
          ? (e as { code: number }).code
          : -32603;
      error = { code, message: e instanceof Error ? e.message : String(e) };
    }

    // Send response back to agent
    const response = error
      ? { jsonrpc: '2.0', id, error }
      : { jsonrpc: '2.0', id, result };
    
    // Track outgoing response
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
    if (method === 'session/update') {
      if (this.onSessionUpdate) {
        this.onSessionUpdate(params as SessionNotification);
      }
    }
  }

  private getTimeoutForMethod(method: string): number {
    switch (method) {
      case 'session/prompt':
      case 'terminal/wait_for_exit':
        return 30 * 60 * 1000; // 30 minutes
      case 'terminal/create':
        return 5 * 60 * 1000; // 5 minutes
      case 'session/load':
        return 5 * 60 * 1000; // 5 minutes
      default:
        return 60 * 1000; // 60 seconds
    }
  }

  private async sendRequest<T>(method: string, params?: unknown): Promise<T> {
    const id = this.nextRequestId++;
    const request = {
      jsonrpc: '2.0',
      id,
      method,
      params: withAcpMeta(params ?? {}),
    };

    // Track outgoing request
    const store = getTrafficStore();
    store.addEntry({
      direction: 'out',
      type: 'request',
      method,
      requestId: id,
      payload: request,
    });
    this.pendingMethods.set(id, method);

    console.log('Sending request:', request);
    const timeoutMs = this.getTimeoutForMethod(method);

    return new Promise((resolve, reject) => {
      this.messageResolvers.set(id, (response) => {
        resolve(response as T);
      });
      this.messageRejecters.set(id, reject);

      sendToAgent(this.agentId, JSON.stringify(request)).catch((e) => {
        this.messageResolvers.delete(id);
        this.messageRejecters.delete(id);
        this.pendingMethods.delete(id);
        reject(e);
      });

      // Apply a method-specific timeout so long-lived ACP flows can complete.
      setTimeout(() => {
        if (this.messageResolvers.has(id)) {
          this.messageResolvers.delete(id);
          this.messageRejecters.delete(id);
          this.pendingMethods.delete(id);
          reject(new Error(`Request timeout: ${method}`));
        }
      }, timeoutMs);
    });
  }

  private async sendNotification(method: string, params?: unknown): Promise<void> {
    const notification = {
      jsonrpc: '2.0',
      method,
      params: withAcpMeta(params ?? {}),
    };
    
    // Track outgoing notification
    const store = getTrafficStore();
    store.addEntry({
      direction: 'out',
      type: 'notification',
      method,
      payload: notification,
    });
    
    await sendToAgent(this.agentId, JSON.stringify(notification));
  }

  // ACP Agent methods (client calls these to talk to agent)
  async initialize(params: InitializeRequest): Promise<InitializeResponse> {
    const response = await this.sendRequest<InitializeResponse>('initialize', {
      ...withAcpMeta(params),
      clientCapabilities: withExtensionCapabilities(params.clientCapabilities),
    });
    this.supportedAgentMethods = readAdvertisedAgentMethods(response.agentCapabilities?._meta);
    return response;
  }

  async newSession(params: NewSessionRequest): Promise<NewSessionResponse> {
    return this.sendRequest<NewSessionResponse>('session/new', params);
  }

  async loadSession(params: LoadSessionRequest): Promise<LoadSessionResponse> {
    return this.sendRequest<LoadSessionResponse>('session/load', params);
  }

  async listSessions(params?: { cwd?: string; cursor?: string }): Promise<{
    sessions: Array<{ sessionId: string; cwd: string; title?: string; updatedAt?: string }>;
    nextCursor?: string;
  }> {
    return await this.sendRequest('session/list', params ?? {});
  }

  async prompt(params: PromptRequest): Promise<PromptResponse> {
    return this.sendRequest<PromptResponse>('session/prompt', params);
  }

  async cancel(params: CancelNotification): Promise<void> {
    await this.sendNotification('session/cancel', params);
  }

  async setMode(params: { sessionId: string; modeId: string }): Promise<void> {
    await this.sendRequest('session/set_mode', params);
  }

  async unstable_setSessionModel(params: { sessionId: string; modelId: string }): Promise<void> {
    await this.sendRequest(SESSION_SET_MODEL_METHOD, params);
  }

  async unstable_setSessionConfigOption(
    params: { sessionId: string; configId: string; value: string }
  ): Promise<{ configOptions: any[] }> {
    return await this.sendRequest<{ configOptions: any[] }>(
      SESSION_SET_CONFIG_OPTION_METHOD,
      params
    );
  }

  async authenticate(params: AuthenticateRequest): Promise<AuthenticateResponse> {
    return this.sendRequest<AuthenticateResponse>('authenticate', params);
  }

  // ACP Client interface methods (agent calls these)
  async requestPermission(
    params: RequestPermissionRequest
  ): Promise<RequestPermissionResponse> {
    return new Promise((resolve) => {
      this.pendingPermissionRequest.value = {
        sessionId: params.sessionId,
        toolCall: {
          toolCallId: params.toolCall.toolCallId,
          title: params.toolCall.title ?? '',
          kind: params.toolCall.kind ?? 'other',
          status: (params.toolCall.status as 'pending' | 'in_progress' | 'completed' | 'failed') ?? 'pending',
          locations: params.toolCall.locations?.map(({ path, line }) =>
            line == null ? { path } : { path, line }
          ),
          content: params.toolCall.content ?? undefined,
          _meta: params.toolCall._meta ?? null,
        },
        options: params.options.map((opt) => ({
          kind: opt.kind,
          name: opt.name,
          optionId: opt.optionId,
          _meta: opt._meta ?? null,
        })),
        _meta: params._meta ?? null,
      };
      this.permissionResolver = resolve;
    });
  }

  resolvePermission(optionId: string): void {
    if (this.permissionResolver) {
      this.permissionResolver({
        outcome: {
          outcome: 'selected',
          optionId,
        },
        _meta: null,
      });
      this.permissionResolver = null;
      this.pendingPermissionRequest.value = null;
    }
  }

  cancelPermission(): void {
    if (this.permissionResolver) {
      this.permissionResolver({
        outcome: {
          outcome: 'cancelled',
        },
        _meta: null,
      });
      this.permissionResolver = null;
      this.pendingPermissionRequest.value = null;
    }
  }

  async sessionUpdate(_params: SessionNotification): Promise<void> {
    // This is called by the agent, we handle it in handleNotification
  }

  async writeTextFile(
    params: WriteTextFileRequest
  ): Promise<WriteTextFileResponse> {
    try {
      assertAbsolutePath(params.path);
      const dir = await dirname(params.path);
      await mkdir(dir, { recursive: true });
      await tauriWriteTextFile(params.path, params.content);
      console.log('writeTextFile completed:', params.path);
      return null as unknown as WriteTextFileResponse;
    } catch (e) {
      console.error('writeTextFile failed:', params.path, e);
      throw e;
    }
  }

  async readTextFile(
    params: ReadTextFileRequest
  ): Promise<ReadTextFileResponse> {
    try {
      assertAbsolutePath(params.path);
      let content = await tauriReadTextFile(params.path);
      
      // Handle line/limit parameters if specified
      if (params.line !== undefined || params.limit !== undefined) {
        const lines = content.split('\n');
        const startLine = params.line ? params.line - 1 : 0; // 1-based to 0-based
        const endLine = params.limit ? startLine + params.limit : lines.length;
        content = lines.slice(startLine, endLine).join('\n');
      }
       
      console.log('readTextFile completed:', params.path);
      return { content, _meta: null };
    } catch (e) {
      console.error('readTextFile failed:', params.path, e);
      throw e;
    }
  }
}

// Factory function to create a connected ACP client
export async function createAcpClient(
  agentInstance: AgentInstance
): Promise<AcpClientBridge> {
  const client = new AcpClientBridge(agentInstance.id);
  await client.connect();
  return client;
}
