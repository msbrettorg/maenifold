import type {
  AuthenticateRequest,
  AuthenticateResponse,
  Client,
  InitializeRequest,
  InitializeResponse,
  LoadSessionRequest,
  LoadSessionResponse,
  NewSessionRequest,
  NewSessionResponse,
  PromptRequest,
  PromptResponse,
} from '@agentclientprotocol/sdk';
import {
  isRecord,
  readAdvertisedAgentMethods,
  withAcpMeta,
  withExtensionCapabilities,
} from './acp-meta';
import { AcpClientBridgeCore } from './acp-client-core';
import { PermissionHandler } from './acp-permissions';
import type { AgentInstance } from './types';

const SESSION_SET_CONFIG_OPTION_METHOD = 'session/set_config_option';
const SESSION_CLOSE_METHOD = 'session/close';

export class AcpClientBridge extends AcpClientBridgeCore implements Client {
  private supportsSessionCloseRequest = false;

  async initialize(params: InitializeRequest): Promise<InitializeResponse> {
    const response = await this.sendRequest<InitializeResponse>('initialize', {
      ...withAcpMeta(params),
      clientCapabilities: withExtensionCapabilities(params.clientCapabilities),
    });
    this.supportedAgentMethods = readAdvertisedAgentMethods(response.agentCapabilities?._meta);
    this.supportsSessionCloseRequest =
      this.supportsAgentMethod(SESSION_CLOSE_METHOD) ||
      Boolean(
        isRecord(response.agentCapabilities) &&
          isRecord(response.agentCapabilities.sessionCapabilities) &&
          'close' in response.agentCapabilities.sessionCapabilities &&
          response.agentCapabilities.sessionCapabilities.close != null
      );
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
    return this.sendRequest('session/list', params ?? {});
  }

  async prompt(params: PromptRequest): Promise<PromptResponse> {
    return this.sendRequest<PromptResponse>('session/prompt', params);
  }

  async unstable_setSessionConfigOption(params: {
    sessionId: string;
    configId: string;
    value: string | boolean;
  }): Promise<{ configOptions: unknown[] }> {
    return this.sendRequest<{ configOptions: unknown[] }>(SESSION_SET_CONFIG_OPTION_METHOD, params);
  }

  supportsSessionClose(): boolean {
    return this.supportsSessionCloseRequest;
  }

  async closeSession(params: { sessionId: string }): Promise<void> {
    await this.sendRequest(SESSION_CLOSE_METHOD, params);
  }

  async authenticate(params: AuthenticateRequest): Promise<AuthenticateResponse> {
    return this.sendRequest<AuthenticateResponse>('authenticate', params);
  }
}

export async function createAcpClient(
  agentInstance: AgentInstance,
  options: { onStderr?: (line: string) => void } = {},
): Promise<AcpClientBridge> {
  const client = new AcpClientBridge(agentInstance.id, agentInstance.name, new PermissionHandler());
  if (options.onStderr) {
    client.onStderr = options.onStderr;
  }
  await client.connect();
  return client;
}
