// Permissions composable — handles permission resolution and auth method selection
import type { Ref } from 'vue';
import type { SessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import type { AuthMethod } from '@agentclientprotocol/sdk';
import { normalizeAuthMethods } from '../lib/auth-methods';

interface UsePermissionsDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
}

export function usePermissions({ sessions, bridges, activeSessionId }: UsePermissionsDeps) {
  const authMethodResolvers = new Map<string, (methodId: string | null) => void>();

  function resolvePermission(optionId: string): void {
    if (!activeSessionId.value) return;
    const session = sessions.value.get(activeSessionId.value);
    const bridge = bridges.get(activeSessionId.value);
    const activePermission = session?.pendingPermissions[0];
    if (bridge && activePermission) {
      bridge.resolvePermission(activePermission.toolCall.toolCallId, optionId);
    }
  }

  function cancelPermission(): void {
    if (!activeSessionId.value) return;
    const bridge = bridges.get(activeSessionId.value);
    if (bridge) {
      bridge.cancelPermission();
    }
  }

  function selectAuthMethod(methodId: string): void {
    const sessionId = activeSessionId.value;
    if (!sessionId) {
      return;
    }

    const resolver = authMethodResolvers.get(sessionId);
    if (resolver) {
      resolver(methodId);
      authMethodResolvers.delete(sessionId);
    }

    const session = sessions.value.get(sessionId);
    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }
  }

  function cancelAuthSelection(): void {
    const sessionId = activeSessionId.value;
    if (!sessionId) {
      return;
    }

    const resolver = authMethodResolvers.get(sessionId);
    if (resolver) {
      resolver(null);
      authMethodResolvers.delete(sessionId);
    }

    const session = sessions.value.get(sessionId);
    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }
  }

  /** Prompt user for auth method selection. Returns the selected method ID or null if cancelled. */
  async function promptForAuthMethod(
    sessionId: string,
    session: SessionState,
    authMethods: AuthMethod[],
    agentName: string
  ): Promise<string | null> {
    session.pendingAuthMethods = [];
    session.pendingAuthAgentName = '';

    const supportedAuthMethods = normalizeAuthMethods(authMethods);
    if (supportedAuthMethods.length === 0) {
      throw new Error('This agent only advertised unsupported authentication methods');
    }

    return new Promise((resolve) => {
      session.pendingAuthMethods = supportedAuthMethods;
      session.pendingAuthAgentName = agentName;
      authMethodResolvers.set(sessionId, resolve);
    });
  }

  /** Cancel an auth resolver by session ID (used by lifecycle's cancelConnection). */
  function cancelAuthResolver(sessionId: string): void {
    const resolver = authMethodResolvers.get(sessionId);
    if (resolver) {
      resolver(null);
      authMethodResolvers.delete(sessionId);
    }
  }

  return {
    resolvePermission,
    cancelPermission,
    selectAuthMethod,
    cancelAuthSelection,
    promptForAuthMethod,
    cancelAuthResolver,
  };
}
