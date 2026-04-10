// Session startup composable — create and resume flows built on shared bootstrap helpers.
import type { SavedSession, SessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { createAcpClient } from '../lib/acp-client';
import { killAgent, onAgentStderr, spawnAgent } from '../lib/tauri';
import { trackError, trackEvent } from '../lib/telemetry';
import type { SessionRuntimeDeps } from './session-runtime-deps';
import { applySessionConfigurationState, detectPhase } from './session-utils';
import { useSessionBootstrap } from './useSessionBootstrap';

interface UseSessionStartupDeps extends SessionRuntimeDeps {
  bindPendingPermission: (sessionId: string, bridge: AcpClientBridge) => void;
  stopPermissionWatch: (sessionId: string) => void;
  createSessionSlot: (
    agentName: string,
    cwd: string,
    options?: { startupPhase?: string }
  ) => { localId: string; sessionState: SessionState };
  clearSessionAbort: (sessionId: string) => void;
  isSessionAborted: (sessionId: string) => boolean;
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
    agentInstance: Awaited<ReturnType<typeof spawnAgent>>
  ): Promise<AcpClientBridge> {
    const bridge = await createAcpClient(agentInstance);
    bridges.set(localId, bridge);
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

  function handleLifecycleFailure(
    localId: string,
    eventName: 'SessionCreated' | 'SessionResumed',
    agentName: string,
    error: unknown
  ): never {
    const session = sessions.value.get(localId);
    if (session) {
      session.error = error instanceof Error ? error.message : String(error);
    }

    stopPermissionWatch(localId);
    bridges.delete(localId);
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

  async function createSession(agentName: string, cwd: string): Promise<void> {
    const { localId, sessionState } = createSessionSlot(agentName, cwd, {
      startupPhase: 'starting',
    });
    let startupTimer: ReturnType<typeof setInterval> | null = null;
    let stderrUnlisten: (() => void) | null = null;

    startupTimer = setInterval(() => {
      const session = sessions.value.get(localId);
      if (session) {
        session.startupElapsed++;
      }
    }, 1000);

    try {
      const agentInstance = await spawnAgent(agentName);
      // Track the Tauri agent process ID so agent-closed events can target this session.
      sessionState.agentId = agentInstance.id;
      stderrUnlisten = await onAgentStderr((stderr) => {
        if (stderr.agent_id !== agentInstance.id) {
          return;
        }

        const session = sessions.value.get(localId);
        if (!session) {
          return;
        }

        session.startupLogs.push(stderr.line);
        const detectedPhase = detectPhase(stderr.line);
        if (detectedPhase) {
          session.startupPhase = detectedPhase;
        }
      });

      await ensureNotAborted(localId, () => killAgent(agentInstance.id));
      sessionState.startupPhase = 'initializing';

      const bridge = await createBoundBridge(localId, agentInstance);
      sessionState.startupPhase = 'connecting';

      const initialized = await initializeConnectedBridge(localId, bridge, sessionState);
      if (import.meta.env.DEV) {
        console.log('Agent initialized:', initialized.initResponse);
      }

      const sessionResponse = await runWithAuthenticationRetry(
        localId,
        bridge,
        sessionState,
        agentName,
        initialized.availableAuthMethods,
        () => bridge.newSession({ cwd, mcpServers: [] }),
        'Authentication cancelled by user'
      );

      sessionState.session = {
        id: crypto.randomUUID(),
        agentName,
        sessionId: sessionResponse.sessionId,
        title: `Session ${new Date().toLocaleString()}`,
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
    } catch (error) {
      handleLifecycleFailure(localId, 'SessionCreated', agentName, error);
    } finally {
      finalizeConnectionAttempt(localId);
      if (startupTimer) {
        clearInterval(startupTimer);
      }
      if (stderrUnlisten) {
        stderrUnlisten();
      }
    }
  }

  async function resumeSession(savedSession: SavedSession): Promise<void> {
    for (const [localId, session] of sessions.value.entries()) {
      if (session.session?.sessionId === savedSession.sessionId) {
        activeSessionId.value = localId;
        return;
      }
    }

    const { localId, sessionState } = createSessionSlot(savedSession.agentName, savedSession.cwd);

    try {
      const agentInstance = await spawnAgent(savedSession.agentName);
      // Track the Tauri agent process ID so agent-closed events can target this session.
      sessionState.agentId = agentInstance.id;
      await ensureNotAborted(localId, () => killAgent(agentInstance.id));

      const bridge = await createBoundBridge(localId, agentInstance);
      const initialized = await initializeConnectedBridge(localId, bridge, sessionState);

      if (!initialized.initResponse.agentCapabilities?.loadSession) {
        await bridge.disconnect();
        throw new Error('Agent no longer supports session loading');
      }

      sessionState.messages = [];
      sessionState.toolCalls.clear();

      const loadResponse = await runWithAuthenticationRetry(
        localId,
        bridge,
        sessionState,
        savedSession.agentName,
        initialized.availableAuthMethods,
        async () => {
          await validateSavedSessionCwd(bridge, savedSession);
          return bridge.loadSession({
            sessionId: savedSession.sessionId,
            cwd: savedSession.cwd,
            mcpServers: [],
          });
        },
        (wasAborted) => (wasAborted ? 'Connection cancelled' : 'Authentication cancelled by user')
      );

      await ensureNotAborted(localId, () => bridge.disconnect());
      sessionState.session = savedSession;
      sessionState.isConnected = true;
      applySessionConfigurationState(sessionState, loadResponse);

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, savedSession.agentName, savedSession.cwd);
      }

      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'true' });
      savedSession.lastUpdated = Date.now();
      await saveSessionsToStore();
    } catch (error) {
      handleLifecycleFailure(localId, 'SessionResumed', savedSession.agentName, error);
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
