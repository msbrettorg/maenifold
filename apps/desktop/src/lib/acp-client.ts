import {
  AGENT_METHODS,
  type Agent,
  AuthenticateRequest,
  AuthenticateResponse,
  InitializeRequest,
  InitializeResponse,
  ListSessionsRequest,
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
import { PermissionHandler, type InlinePermissionMessageSink } from './acp-permissions';
import type { AgentInstance } from './types';

const SESSION_SET_CONFIG_OPTION_METHOD = AGENT_METHODS.session_set_config_option;
const SESSION_CLOSE_METHOD = AGENT_METHODS.session_close;

export class AcpClientBridge extends AcpClientBridgeCore implements Agent {
  private supportsSessionCloseRequest = false;

  async initialize(params: InitializeRequest): Promise<InitializeResponse> {
    const response = await this.sendRequest<InitializeResponse>(AGENT_METHODS.initialize, {
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
    return this.sendRequest<NewSessionResponse>(AGENT_METHODS.session_new, params);
  }

  async loadSession(params: LoadSessionRequest): Promise<LoadSessionResponse> {
    return this.sendRequest<LoadSessionResponse>(AGENT_METHODS.session_load, params);
  }

  async listSessions(params?: ListSessionsRequest): Promise<{
    sessions: Array<{ sessionId: string; cwd: string; title?: string; updatedAt?: string }>;
    nextCursor?: string;
  }> {
    return this.sendRequest(AGENT_METHODS.session_list, params ?? {});
  }

  async prompt(params: PromptRequest): Promise<PromptResponse> {
    return this.sendRequest<PromptResponse>(AGENT_METHODS.session_prompt, params);
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
    return this.sendRequest<AuthenticateResponse>(AGENT_METHODS.authenticate, params);
  }
}

export async function createAcpClient(
  agentInstance: AgentInstance,
  options: {
    onStderr?: (line: string) => void;
    onInlineMessage?: InlinePermissionMessageSink;
  } = {},
): Promise<AcpClientBridge> {
  const client = new AcpClientBridge(
    agentInstance.id,
    agentInstance.name,
    new PermissionHandler(undefined, {
      agent: agentInstance.name,
      onInlineMessage: options.onInlineMessage,
    })
  );
  if (options.onStderr) {
    client.onStderr = options.onStderr;
  }
  await client.connect();
  return client;
}
