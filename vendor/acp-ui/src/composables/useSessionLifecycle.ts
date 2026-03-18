// Session lifecycle composable — create, resume, disconnect, close sessions
import { reactive, watch, type Ref, type WatchStopHandle } from 'vue';
import type { SavedSession, SessionState } from '../lib/types';
import { generateSessionId, createEmptySessionState } from '../lib/types';
import { AcpClientBridge, createAcpClient, isAbsolutePath } from '../lib/acp-bridge';
import { spawnAgent, killAgent, onAgentStderr } from '../lib/tauri';
import { trackEvent, trackError } from '../lib/telemetry';
import type { AuthMethod, SessionNotification } from '@agentclientprotocol/sdk';
import { detectPhase, applySessionConfigurationState } from './session-utils';

const PROTOCOL_VERSION = 1;

interface UseSessionLifecycleDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
  mergeSessionsFromAgent: (
    bridge: AcpClientBridge,
    agentName: string,
    cwd: string
  ) => Promise<void>;
  handleSessionUpdate: (
    localSessionId: string,
    notification: SessionNotification
  ) => void;
  promptForAuthMethod: (
    sessionId: string,
    session: SessionState,
    authMethods: AuthMethod[],
    agentName: string
  ) => Promise<string | null>;
  cancelAuthResolver: (sessionId: string) => void;
  getAppVersion: () => string;
}

export function useSessionLifecycle({
  sessions,
  bridges,
  activeSessionId,
  savedSessions,
  saveSessionsToStore,
  mergeSessionsFromAgent,
  handleSessionUpdate,
  promptForAuthMethod,
  cancelAuthResolver,
  getAppVersion,
}: UseSessionLifecycleDeps) {
  const abortedSessions = new Set<string>();
  const permissionWatchStops = new Map<string, WatchStopHandle>();

  // ── Internal helpers ──────────────────────────────────────────────

  function replaceSessionsMap(): void {
    sessions.value = new Map(sessions.value);
  }

  function setSessionState(sessionId: string, sessionState: SessionState): void {
    sessions.value.set(sessionId, sessionState);
    replaceSessionsMap();
  }

  function deleteSessionState(sessionId: string): void {
    sessions.value.delete(sessionId);
    replaceSessionsMap();
  }

  function adjustActiveSessionId(removedId: string): void {
    if (activeSessionId.value === removedId) {
      const remaining = Array.from(sessions.value.keys());
      activeSessionId.value = remaining.length > 0 ? remaining[0] : null;
    }
  }

  function stopPermissionWatch(sessionId: string): void {
    const stop = permissionWatchStops.get(sessionId);
    if (stop) {
      stop();
      permissionWatchStops.delete(sessionId);
    }
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

  function bindPendingPermission(sessionId: string, bridge: AcpClientBridge): void {
    stopPermissionWatch(sessionId);
    const stop = watch(
      () => bridge.pendingPermissionRequest.value,
      (value) => {
        const session = sessions.value.get(sessionId);
        if (session) {
          session.pendingPermissions = value ? [value] : [];
        }
      },
      { immediate: true }
    );
    permissionWatchStops.set(sessionId, stop);
  }

  async function performInitialization(client: AcpClientBridge) {
    const initResponse = await client.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: {
        fs: {
          readTextFile: true,
          writeTextFile: true,
        },
      },
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

    const agentInfoResult = initResponse.agentInfo
      ? {
          name: initResponse.agentInfo.name,
          title: initResponse.agentInfo.title ?? undefined,
          version: initResponse.agentInfo.version ?? undefined,
          _meta:
            typeof initResponse.agentInfo._meta === 'object' && initResponse.agentInfo._meta !== null
              ? initResponse.agentInfo._meta
              : null,
        }
      : null;
    const promptCaps = initResponse.agentCapabilities?.promptCapabilities
      ? {
          ...initResponse.agentCapabilities.promptCapabilities,
          _meta:
            typeof initResponse.agentCapabilities.promptCapabilities._meta === 'object' &&
            initResponse.agentCapabilities.promptCapabilities._meta !== null
              ? initResponse.agentCapabilities.promptCapabilities._meta
              : null,
        }
      : null;

    return {
      initResponse,
      availableAuthMethods: initResponse.authMethods || [],
      agentInfoResult,
      promptCaps,
    };
  }

  // ── Create new session ────────────────────────────────────────────

  async function createSession(agentName: string, cwd: string): Promise<void> {
    // Clean up previous non-connected active session (e.g., from a failed attempt)
    if (activeSessionId.value) {
      const existing = sessions.value.get(activeSessionId.value);
      if (existing && !existing.isConnected) {
        deleteSessionState(activeSessionId.value);
        bridges.delete(activeSessionId.value);
      }
    }

    // Create session slot
    const localId = generateSessionId();
    const sessionState = reactive(createEmptySessionState(localId, agentName, cwd));
    sessionState.isLoading = true;
    sessionState.isConnecting = true;
    sessionState.currentPlan = null;
    sessionState.startupPhase = 'starting';
    setSessionState(localId, sessionState);
    activeSessionId.value = localId;

    abortedSessions.delete(localId);

    // Startup progress tracking (local to this call)
    let startupTimer: ReturnType<typeof setInterval> | null = null;
    let stderrUnlisten: (() => void) | null = null;

    startupTimer = setInterval(() => {
      const s = sessions.value.get(localId);
      if (s) s.startupElapsed++;
    }, 1000);

    try {
      // Spawn agent process
      const agentInstance = await spawnAgent(agentName);

      // Listen for stderr to track startup progress
      stderrUnlisten = await onAgentStderr((stderr) => {
        if (stderr.agent_id === agentInstance.id) {
          const s = sessions.value.get(localId);
          if (s) {
            s.startupLogs.push(stderr.line);
            const detectedPhase = detectPhase(stderr.line);
            if (detectedPhase) {
              s.startupPhase = detectedPhase;
            }
          }
        }
      }) as unknown as () => void;

      if (abortedSessions.has(localId)) {
        await killAgent(agentInstance.id);
        throw new Error('Connection cancelled');
      }

      sessionState.startupPhase = 'initializing';

      // Create ACP client bridge and store in bridges Map
      const bridge = await createAcpClient(agentInstance);
      bridges.set(localId, bridge);
      bindPendingPermission(localId, bridge);

      // Wire scoped session update handler
      bridge.onSessionUpdate = (notification) =>
        handleSessionUpdate(localId, notification);

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      sessionState.startupPhase = 'connecting';

      // Initialize connection
      const { initResponse, availableAuthMethods, agentInfoResult, promptCaps } =
        await performInitialization(bridge);

      console.log('Agent initialized:', initResponse);

      sessionState.agentInfo = agentInfoResult;
      sessionState.agentPromptCapabilities = promptCaps;

      // Check if agent supports session loading
      const supportsLoadSession = initResponse.agentCapabilities?.loadSession ?? false;
      sessionState.supportsSessionList = Boolean(
        initResponse.agentCapabilities?.sessionCapabilities?.list
      );

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      // Try to create session - may fail with auth_required
      let sessionResponse;
      try {
        sessionResponse = await bridge.newSession({
          cwd,
          mcpServers: [],
        });
      } catch (sessionError: unknown) {
        // Check if auth is required (error code -32000)
        const errorMessage = sessionError instanceof Error ? sessionError.message : String(sessionError);
        const isAuthRequired = errorMessage.toLowerCase().includes('authentication required') ||
                               errorMessage.includes('-32000');

        if (isAuthRequired && availableAuthMethods.length > 0) {
          console.log('Authentication required, available methods:', availableAuthMethods);

          // Prompt user to select auth method
          const selectedMethodId = await promptForAuthMethod(
            localId,
            sessionState, availableAuthMethods, agentName
          );

          if (!selectedMethodId || abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error('Authentication cancelled by user');
          }

          console.log('Authenticating with method:', selectedMethodId);
          const authResponse = await bridge.authenticate({
            methodId: selectedMethodId,
          });
          console.log('Authentication successful:', authResponse);

          if (abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error('Connection cancelled');
          }

          // Retry session creation after auth
          sessionResponse = await bridge.newSession({
            cwd,
            mcpServers: [],
          });
        } else {
          throw sessionError;
        }
      }

      // Save session
      const saved: SavedSession = {
        id: crypto.randomUUID(),
        agentName,
        sessionId: sessionResponse.sessionId,
        title: `Session ${new Date().toLocaleString()}`,
        lastUpdated: Date.now(),
        cwd,
        supportsLoadSession,
      };

      sessionState.session = saved;
      savedSessions.value.push(saved);
      await saveSessionsToStore();

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, agentName, cwd);
      }

      sessionState.isConnected = true;
      sessionState.messages = [];
      sessionState.toolCalls.clear();

      // Track successful session creation
      trackEvent('SessionCreated', { agentName, success: 'true' });

      applySessionConfigurationState(sessionState, sessionResponse);

    } catch (e) {
      const s = sessions.value.get(localId);
      if (s) {
        s.error = e instanceof Error ? e.message : String(e);
      }
      stopPermissionWatch(localId);
      bridges.delete(localId);
      // Track session creation failure
      trackEvent('SessionCreated', { agentName, success: 'false' });
      trackError(e instanceof Error ? e : new Error(String(e)));
      throw e;
    } finally {
      const s = sessions.value.get(localId);
      if (s) {
        s.isLoading = false;
        s.isConnecting = false;
      }
      abortedSessions.delete(localId);
      // Clean up startup progress tracking
      if (startupTimer) {
        clearInterval(startupTimer);
        startupTimer = null;
      }
      if (stderrUnlisten) {
        stderrUnlisten();
        stderrUnlisten = null;
      }
    }
  }

  // ── Resume existing session ───────────────────────────────────────

  async function resumeSession(savedSession: SavedSession): Promise<void> {
    for (const [localId, session] of sessions.value.entries()) {
      if (session.session?.sessionId === savedSession.sessionId) {
        activeSessionId.value = localId;
        return;
      }
    }

    // Clean up previous non-connected active session
    if (activeSessionId.value) {
      const existing = sessions.value.get(activeSessionId.value);
      if (existing && !existing.isConnected) {
        deleteSessionState(activeSessionId.value);
        bridges.delete(activeSessionId.value);
      }
    }

    // Create session slot
    const localId = generateSessionId();
    const sessionState = reactive(
      createEmptySessionState(localId, savedSession.agentName, savedSession.cwd)
    );
    sessionState.isLoading = true;
    sessionState.isConnecting = true;
    sessionState.currentPlan = null;
    setSessionState(localId, sessionState);
    activeSessionId.value = localId;

    abortedSessions.delete(localId);

    try {
      // Spawn agent process
      const agentInstance = await spawnAgent(savedSession.agentName);

      if (abortedSessions.has(localId)) {
        await killAgent(agentInstance.id);
        throw new Error('Connection cancelled');
      }

      // Create ACP client bridge and store in bridges Map
      const bridge = await createAcpClient(agentInstance);
      bridges.set(localId, bridge);
      bindPendingPermission(localId, bridge);

      // Wire scoped session update handler
      bridge.onSessionUpdate = (notification) =>
        handleSessionUpdate(localId, notification);

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      // Initialize connection
      const { initResponse, availableAuthMethods, agentInfoResult, promptCaps } =
        await performInitialization(bridge);

      sessionState.agentInfo = agentInfoResult;
      sessionState.agentPromptCapabilities = promptCaps;

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      sessionState.supportsSessionList = Boolean(
        initResponse.agentCapabilities?.sessionCapabilities?.list
      );

      if (!initResponse.agentCapabilities?.loadSession) {
        await bridge.disconnect();
        throw new Error('Agent no longer supports session loading');
      }

      // Clear messages BEFORE loadSession - the agent will stream replay via notifications
      sessionState.messages = [];
      sessionState.toolCalls.clear();

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      // Try to load existing session - may fail with auth_required
      let loadResponse;
      try {
        await validateSavedSessionCwd(bridge, savedSession);
        loadResponse = await bridge.loadSession({
          sessionId: savedSession.sessionId,
          cwd: savedSession.cwd,
          mcpServers: [],
        });
      } catch (sessionError: unknown) {
        // Check if auth is required (error code -32000)
        const errorMessage = sessionError instanceof Error ? sessionError.message : String(sessionError);
        const isAuthRequired = errorMessage.toLowerCase().includes('authentication required') ||
                               errorMessage.includes('-32000');

        if (isAuthRequired && availableAuthMethods.length > 0) {
          console.log('Authentication required, available methods:', availableAuthMethods);

          // Prompt user to select auth method
          const selectedMethodId = await promptForAuthMethod(
            localId,
            sessionState, availableAuthMethods, savedSession.agentName
          );

          if (!selectedMethodId || abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error(
              abortedSessions.has(localId)
                ? 'Connection cancelled'
                : 'Authentication cancelled by user'
            );
          }

          console.log('Authenticating with method:', selectedMethodId);
          const authResponse = await bridge.authenticate({
            methodId: selectedMethodId,
          });
          console.log('Authentication successful:', authResponse);

          if (abortedSessions.has(localId)) {
            await bridge.disconnect();
            throw new Error('Connection cancelled');
          }

          // Retry loading session after auth
          await validateSavedSessionCwd(bridge, savedSession);
          loadResponse = await bridge.loadSession({
            sessionId: savedSession.sessionId,
            cwd: savedSession.cwd,
            mcpServers: [],
          });
        } else {
          throw sessionError;
        }
      }

      if (abortedSessions.has(localId)) {
        await bridge.disconnect();
        throw new Error('Connection cancelled');
      }

      sessionState.session = savedSession;
      sessionState.isConnected = true;
      // Messages already populated by session/update notifications during loadSession
      applySessionConfigurationState(sessionState, loadResponse);

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, savedSession.agentName, savedSession.cwd);
      }

      // Track successful session resume
      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'true' });

      // Update last accessed time
      savedSession.lastUpdated = Date.now();
      await saveSessionsToStore();

    } catch (e) {
      const s = sessions.value.get(localId);
      if (s) {
        s.error = e instanceof Error ? e.message : String(e);
      }
      stopPermissionWatch(localId);
      bridges.delete(localId);
      // Track session resume failure
      trackEvent('SessionResumed', { agentName: savedSession.agentName, success: 'false' });
      trackError(e instanceof Error ? e : new Error(String(e)));
      throw e;
    } finally {
      const s = sessions.value.get(localId);
      if (s) {
        s.isLoading = false;
        s.isConnecting = false;
      }
      abortedSessions.delete(localId);
    }
  }

  // ── Disconnect session ────────────────────────────────────────────

  async function disconnect(sessionId?: string): Promise<void> {
    const targetId = sessionId ?? activeSessionId.value;
    if (!targetId) return;

    const session = sessions.value.get(targetId);
    const bridge = bridges.get(targetId);

    const agentName = session?.session?.agentName || session?.agentName || 'unknown';
    const sessionStart = session?.session?.lastUpdated || Date.now();
    const sessionDuration = Math.round((Date.now() - sessionStart) / 1000);

    if (session) {
      session.currentPlan = null;
    }

    if (bridge) {
      await bridge.disconnect();
      stopPermissionWatch(targetId);
      bridges.delete(targetId);
    }

    // Track session disconnect
    trackEvent('SessionDisconnected', {
      agentName,
      sessionDurationSeconds: String(sessionDuration),
      messageCount: String(session?.messages.length ?? 0),
    });

    deleteSessionState(targetId);
    adjustActiveSessionId(targetId);
  }

  // ── Set active session ────────────────────────────────────────────

  function setActiveSession(sessionId: string): void {
    if (sessions.value.has(sessionId)) {
      activeSessionId.value = sessionId;
    }
  }

  // ── Close session ─────────────────────────────────────────────────

  async function closeSession(sessionId: string): Promise<void> {
    const session = sessions.value.get(sessionId);
    if (!session) return;

    if (session.isConnected || session.isConnecting) {
      await disconnect(sessionId);
    } else {
      stopPermissionWatch(sessionId);
      bridges.delete(sessionId);
      deleteSessionState(sessionId);
      adjustActiveSessionId(sessionId);
    }
  }

  // ── Cancel ongoing connection attempt ─────────────────────────────

  async function cancelConnection(): Promise<void> {
    const targetId = activeSessionId.value;
    if (!targetId) return;
    abortedSessions.add(targetId);

    // Cancel auth selection if pending
    cancelAuthResolver(targetId);

    const session = sessions.value.get(targetId);
    const bridge = bridges.get(targetId);

    if (session) {
      session.pendingAuthMethods = [];
      session.pendingAuthAgentName = '';
    }

    // Disconnect if bridge exists
    if (bridge) {
      try {
        await bridge.disconnect();
      } catch (e) {
        console.error('Error disconnecting:', e);
      }
      stopPermissionWatch(targetId);
      bridges.delete(targetId);
    }

    deleteSessionState(targetId);
    adjustActiveSessionId(targetId);
  }

  return {
    performInitialization,
    createSession,
    resumeSession,
    disconnect,
    closeSession,
    setActiveSession,
    cancelConnection,
  };
}
