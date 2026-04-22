// Session bootstrap composable — shared initialization and auth retry helpers.
import type { PromptCapabilities, SavedSession, SessionState } from '../lib/types';
import type { AgentInfo } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { isAbsolutePath } from '../lib/acp-meta';
import type { AuthMethod, InitializeRequest } from '@agentclientprotocol/sdk';

const PROTOCOL_VERSION = 1;

type InitializationResult = {
  initResponse: Awaited<ReturnType<AcpClientBridge['initialize']>>;
  availableAuthMethods: AuthMethod[];
  agentInfoResult: AgentInfo | null;
  promptCaps: PromptCapabilities | null;
};

interface UseSessionBootstrapDeps {
  getAppVersion: () => string;
  promptForAuthMethod: (
    sessionId: string,
    session: SessionState,
    authMethods: AuthMethod[],
    agentName: string
  ) => Promise<string | null>;
  isSessionAborted: (sessionId: string) => boolean;
}

export function useSessionBootstrap({
  getAppVersion,
  promptForAuthMethod,
  isSessionAborted,
}: UseSessionBootstrapDeps) {
  async function ensureNotAborted(
    sessionId: string,
    cleanup?: () => Promise<void> | void
  ): Promise<void> {
    if (!isSessionAborted(sessionId)) {
      return;
    }

    await cleanup?.();
    throw new Error('Connection cancelled');
  }

  async function validateSavedSessionCwd(
    bridge: AcpClientBridge,
    savedSession: SavedSession
  ): Promise<void> {
    if (!isAbsolutePath(savedSession.cwd)) {
      await bridge.disconnect();
      throw new Error('Saved session working directory must be an absolute path');
    }
  }

  async function performInitialization(client: AcpClientBridge): Promise<InitializationResult> {
    type InitializeClientCapabilities = NonNullable<InitializeRequest['clientCapabilities']> & {
      prompt: PromptCapabilities;
    };

    const clientCapabilities: InitializeClientCapabilities = {
      terminal: false,
      prompt: { image: true, audio: true, embeddedContext: true },
    };

    const initResponse = await client.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities,
      clientInfo: {
        name: 'maenifold',
        title: 'maenifold',
        version: getAppVersion(),
      },
    });

    if (initResponse.protocolVersion !== PROTOCOL_VERSION) {
      await client.disconnect();
      throw new Error(
        `ACP protocol version mismatch: agent responded with version ${initResponse.protocolVersion}, expected ${PROTOCOL_VERSION}`
      );
    }

    return {
      initResponse,
      availableAuthMethods: initResponse.authMethods || [],
      agentInfoResult: initResponse.agentInfo
        ? {
            name: initResponse.agentInfo.name,
            title: initResponse.agentInfo.title ?? undefined,
            version: initResponse.agentInfo.version ?? undefined,
            _meta:
              typeof initResponse.agentInfo._meta === 'object' &&
              initResponse.agentInfo._meta !== null
                ? initResponse.agentInfo._meta
                : null,
          }
        : null,
      promptCaps: initResponse.agentCapabilities?.promptCapabilities
        ? {
            ...initResponse.agentCapabilities.promptCapabilities,
            _meta:
              typeof initResponse.agentCapabilities.promptCapabilities._meta === 'object' &&
              initResponse.agentCapabilities.promptCapabilities._meta !== null
                ? initResponse.agentCapabilities.promptCapabilities._meta
                : null,
          }
        : null,
    };
  }

  async function runWithAuthenticationRetry<T>(
    localId: string,
    bridge: AcpClientBridge,
    sessionState: SessionState,
    agentName: string,
    authMethods: AuthMethod[],
    operation: () => Promise<T>,
    cancelledMessage: string | ((aborted: boolean) => string)
  ): Promise<T> {
    try {
      return await operation();
    } catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      const isAuthRequired =
        errorMessage.toLowerCase().includes('authentication required') ||
        errorMessage.includes('-32000');

      if (!isAuthRequired || authMethods.length === 0) {
        throw error;
      }

      if (import.meta.env.DEV) {
        console.log('Authentication required, available methods:', authMethods); // audit-ok: no-console
      }

      const selectedMethodId = await promptForAuthMethod(
        localId,
        sessionState,
        authMethods,
        agentName
      );
      const wasAborted = isSessionAborted(localId);
      if (!selectedMethodId || wasAborted) {
        await bridge.disconnect();
        throw new Error(
          typeof cancelledMessage === 'function'
            ? cancelledMessage(wasAborted)
            : cancelledMessage
        );
      }

      if (import.meta.env.DEV) {
        console.log('Authenticating with method:', selectedMethodId); // audit-ok: no-console
      }

      const authResponse = await bridge.authenticate({ methodId: selectedMethodId });
      if (import.meta.env.DEV) {
        console.log('Authentication successful:', authResponse); // audit-ok: no-console
      }

      await ensureNotAborted(localId, () => bridge.disconnect());
      return operation();
    }
  }

  return {
    ensureNotAborted,
    validateSavedSessionCwd,
    performInitialization,
    runWithAuthenticationRetry,
  };
}
