// Session startup composable — create and resume flows built on shared bootstrap helpers.
import type { AcpClientBridge } from '../lib/acp-client';
import { createAcpClient } from '../lib/acp-client';
import type { AgentInstance, SavedSession, SessionSource, SessionState } from '../lib/types';
import { trackError, trackEvent } from '../lib/telemetry';
import type { SessionRuntimeDeps } from './session-runtime-deps';
import {
  applySessionConfigurationState,
  createDefaultSessionTitle,
  detectPhase,
} from './session-utils';
import { useSessionBootstrap } from './useSessionBootstrap';
import { createWritePermissionPolicy } from './useWritePermissionBridge';

interface UseSessionStartupDeps extends SessionRuntimeDeps {
  bindPendingPermission: (sessionId: string, bridge: AcpClientBridge) => void;
  stopPermissionWatch: (sessionId: string) => void;
  createSessionSlot: (
    agentName: string,
    cwd: string,
    options?: { startupPhase?: string; source?: SessionSource }
  ) => { localId: string; sessionState: SessionState };
  clearSessionAbort: (sessionId: string) => void;
  isSessionAborted: (sessionId: string) => boolean;
}

function createAgentInstance(name: string): AgentInstance {
  return {
    id: crypto.randomUUID(),
    name,
  };
}

function isTransientResumeTransportError(error: unknown, bridge: AcpClientBridge): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return bridge.isTransportClosed()
    || message.includes('ACP connection closed')
    || message.includes('ACP client disconnected before response')
    || message.includes('ACP agent terminated before responding')
    || message.includes('ACP relay socket is not connected');
}

function waitForResumeRetry(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 1000));
}

export function useSessionStartup({
  sessions,
  bridges,
  activeSessionId,
  savedSessions,
  saveSessionsToStore,
  mergeSessionsFromAgent,
  handleSessionUpdate,
  promptForAuthMethod,
  getAppVersion,
  clearConnectionError,
  bindPendingPermission,
  stopPermissionWatch,
  createSessionSlot,
  clearSessionAbort,
  isSessionAborted,
}: UseSessionStartupDeps) {
  const {
    ensureNotAborted,
    validateSavedSessionCwd,
    performInitialization,
    runWithAuthenticationRetry,
  } = useSessionBootstrap({
    getAppVersion,
    promptForAuthMethod,
    isSessionAborted,
  });

  async function createBoundBridge(
    localId: string,
    agentInstance: AgentInstance
  ): Promise<AcpClientBridge> {
    let bridge: AcpClientBridge;
    const writePolicy = createWritePermissionPolicy(localId, sessions, () => bridge);
    bridge = await createAcpClient(agentInstance, {
      onStderr: (line) => {
        const session = sessions.value.get(localId);
        if (!session) {
          return;
        }

        session.startupLogs.push(line);
        const detectedPhase = detectPhase(line);
        if (detectedPhase) {
          session.startupPhase = detectedPhase;
        }
      },
      onInlineMessage: (message) => {
        const session = sessions.value.get(localId);
        if (session) {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'system',
            content: message.message,
            timestamp: Date.now(),
          });
        }
      },
      writePolicy,
    });
    bridges.set(localId, bridge);
    bridge.setPermissionModeProvider(() => sessions.value.get(localId)?.currentModeId ?? null);
    bindPendingPermission(localId, bridge);
    bridge.onSessionUpdate = (notification) => handleSessionUpdate(localId, notification);
    await ensureNotAborted(localId, () => bridge.disconnect());
    return bridge;
  }

  async function initializeConnectedBridge(
    localId: string,
    bridge: AcpClientBridge,
    sessionState: SessionState
  ) {
    const initialized = await performInitialization(bridge);
    sessionState.agentInfo = initialized.agentInfoResult;
    sessionState.agentPromptCapabilities = initialized.promptCaps;
    sessionState.supportsSessionList = Boolean(
      initialized.initResponse.agentCapabilities?.sessionCapabilities?.list
    );
    await ensureNotAborted(localId, () => bridge.disconnect());
    return initialized;
  }

  async function handleLifecycleFailure(
    localId: string,
    eventName: 'SessionCreated' | 'SessionResumed',
    agentName: string,
    error: unknown
  ): Promise<never> {
    const session = sessions.value.get(localId);
    if (session) {
      session.error = error instanceof Error ? error.message : String(error);
    }

    stopPermissionWatch(localId);
    const bridge = bridges.get(localId);
    bridges.delete(localId);
    if (bridge) {
      try {
        await bridge.disconnect();
      } catch (disconnectError) {
        const cleanupError = disconnectError instanceof Error
          ? disconnectError
          : new Error(String(disconnectError));
        if (session) {
          session.error = `${session.error ?? 'Connection failed'}; cleanup failed: ${cleanupError.message}`;
        }
        trackError(cleanupError);
      }
    }
    trackEvent(eventName, { agentName, success: 'false' });
    trackError(error instanceof Error ? error : new Error(String(error)));
    throw error;
  }

  function finalizeConnectionAttempt(localId: string): void {
    const session = sessions.value.get(localId);
    if (session) {
      session.isLoading = false;
      session.isConnecting = false;
    }
    clearSessionAbort(localId);
  }

  async function createSession(
    agentName: string,
    cwd: string,
    configOptions: Record<string, string> = {},
    options: { source?: SessionSource } = {},
  ): Promise<void> {
    const { localId, sessionState } = createSessionSlot(agentName, cwd, {
      startupPhase: 'starting',
      source: options.source ?? 'agents',
    });
    let startupTimer: ReturnType<typeof setInterval> | null = null;

    startupTimer = setInterval(() => {
      const session = sessions.value.get(localId);
      if (session) {
        session.startupElapsed++;
      }
    }, 1000);

    try {
      const agentInstance = createAgentInstance(agentName);
      sessionState.agentId = agentInstance.id;
      sessionState.startupPhase = 'initializing';

      const bridge = await createBoundBridge(localId, agentInstance);
      sessionState.startupPhase = 'connecting';

      const initialized = await initializeConnectedBridge(localId, bridge, sessionState);
      if (import.meta.env.DEV) {
      }

      const startupConfigOptions = Object.keys(configOptions).length > 0
        ? { configOptions }
        : {};
      const sessionResponse = await runWithAuthenticationRetry(
        localId,
        bridge,
        sessionState,
        agentName,
        initialized.availableAuthMethods,
        () => bridge.newSession({ cwd, mcpServers: [], ...startupConfigOptions }),
        'Authentication cancelled by user'
      );

      sessionState.session = {
        id: crypto.randomUUID(),
        agentName,
        sessionId: sessionResponse.sessionId,
        title: createDefaultSessionTitle(agentName, savedSessions.value),
        lastUpdated: Date.now(),
        cwd,
        supportsLoadSession: initialized.initResponse.agentCapabilities?.loadSession ?? false,
      };
      sessionState.isConnected = true;
      sessionState.messages = [];
      sessionState.toolCalls.clear();
      savedSessions.value.push(sessionState.session);
      await saveSessionsToStore();

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, agentName, cwd);
      }

      trackEvent('SessionCreated', { agentName, success: 'true' });
      applySessionConfigurationState(sessionState, sessionResponse);
      clearConnectionError();
    } catch (error) {
      await handleLifecycleFailure(localId, 'SessionCreated', agentName, error);
    } finally {
      finalizeConnectionAttempt(localId);
      if (startupTimer) {
        clearInterval(startupTimer);
      }
    }
  }

  async function resumeSession(savedSession: SavedSession): Promise<void> {
    for (const [localId, session] of sessions.value.entries()) {
      if (session.session?.sessionId === savedSession.sessionId && session.isConnected) {
        activeSessionId.value = localId;
        return;
      }
    }

    const { localId, sessionState } = createSessionSlot(savedSession.agentName, savedSession.cwd);

    try {
      let bridge: AcpClientBridge | null = null;
      let initialized: Awaited<ReturnType<typeof initializeConnectedBridge>> | null = null;
      let loadResponse: Awaited<ReturnType<AcpClientBridge['loadSession']>> | null = null;
      let lastError: unknown = null;

      for (let attempt = 0; attempt < 2; attempt += 1) {
        const agentInstance = createAgentInstance(savedSession.agentName);
        sessionState.agentId = agentInstance.id;
        bridge = await createBoundBridge(localId, agentInstance);
        initialized = await initializeConnectedBridge(localId, bridge, sessionState);

        if (!initialized.initResponse.agentCapabilities?.loadSession) {
          await bridge.disconnect();
          throw new Error('Agent no longer supports session loading');
        }

        sessionState.messages = [];
        sessionState.toolCalls.clear();

        try {
          loadResponse = await runWithAuthenticationRetry(
            localId,
            bridge,
            sessionState,
            savedSession.agentName,
            initialized.availableAuthMethods,
            async () => {
              await validateSavedSessionCwd(bridge as AcpClientBridge, savedSession);
              return (bridge as AcpClientBridge).loadSession({
                sessionId: savedSession.sessionId,
                cwd: savedSession.cwd,
                mcpServers: [],
              });
            },
            (wasAborted) => (wasAborted ? 'Connection cancelled' : 'Authentication cancelled by user')
          );
          break;
        } catch (error) {
          lastError = error;
          const shouldRetry = attempt === 0 && isTransientResumeTransportError(error, bridge);
          bridges.delete(localId);
          await bridge.disconnect().catch((disconnectError) => {
            trackError(disconnectError instanceof Error ? disconnectError : new Error(String(disconnectError)));
          });
          bridge = null;
          initialized = null;
          if (!shouldRetry) {
            throw error;
          }
          await waitForResumeRetry();
        }
      }

      if (!bridge || !initialized || !loadResponse) {
        throw lastError ?? new Error('Session resume failed before session/load completed');
      }

      await ensureNotAborted(localId, () => bridge.disconnect());
      sessionState.session = savedSession;
      sessionState.isConnected = true;
      applySessionConfigurationState(sessionState, loadResponse);
      if (sessionState.messages.length === 0 && savedSession.messages?.length) {
        sessionState.messages = savedSession.messages.map((message) => ({
          ...message,
          blocks: message.blocks?.map((block) => ({ ...block })),
          toolCalls: message.toolCalls?.map((toolCall) => ({
            ...toolCall,
            locations: toolCall.locations?.map((location) => ({ ...location })),
            content: toolCall.content?.map((block) => ({ ...block })),
            rawInput: toolCall.rawInput ? { ...toolCall.rawInput } : undefined,
            rawOutput: toolCall.rawOutput ? { ...toolCall.rawOutput } : undefined,
          })),
        }));
      }

      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'true' });
      savedSession.lastUpdated = Date.now();
      await saveSessionsToStore();
      clearConnectionError();
    } catch (error) {
      await handleLifecycleFailure(localId, 'SessionResumed', savedSession.agentName, error);
    } finally {
      finalizeConnectionAttempt(localId);
    }
  }

  return {
    performInitialization,
    createSession,
    resumeSession,
  };
}
