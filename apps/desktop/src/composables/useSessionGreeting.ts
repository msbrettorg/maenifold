import type { AuthMethod } from "@agentclientprotocol/sdk";
import type { AcpClientBridge } from "../lib/acp-client";
import { createAcpClient } from "../lib/acp-client";
import type {
  AgentInfo,
  AgentInstance,
  PromptCapabilities,
  SessionSource,
  SessionState,
} from "../lib/types";
import { trackError, trackEvent } from "../lib/telemetry";
import type { SessionRuntimeDeps } from "./session-runtime-deps";
import {
  applySessionConfigurationState,
  createDefaultSessionTitle,
  detectPhase,
} from "./session-utils";
import { createWritePermissionPolicy } from "./useWritePermissionBridge";

type InitializationResult = {
  initResponse: Awaited<ReturnType<AcpClientBridge["initialize"]>>;
  availableAuthMethods: AuthMethod[];
  agentInfoResult: AgentInfo | null;
  promptCaps: PromptCapabilities | null;
};

interface SessionBootstrapApi {
  ensureNotAborted: (
    sessionId: string,
    cleanup?: () => Promise<void> | void,
  ) => Promise<void>;
  performInitialization: (
    client: AcpClientBridge,
  ) => Promise<InitializationResult>;
  runWithAuthenticationRetry: <T>(
    localId: string,
    bridge: AcpClientBridge,
    sessionState: SessionState,
    agentName: string,
    authMethods: AuthMethod[],
    operation: () => Promise<T>,
    cancelledMessage: string | ((aborted: boolean) => string),
  ) => Promise<T>;
}

interface UseSessionGreetingDeps extends SessionRuntimeDeps {
  bindPendingPermission: (sessionId: string, bridge: AcpClientBridge) => void;
  stopPermissionWatch: (sessionId: string) => void;
  createSessionSlot: (
    agentName: string,
    cwd: string,
    options?: { startupPhase?: string; source?: SessionSource },
  ) => { localId: string; sessionState: SessionState };
  clearSessionAbort: (sessionId: string) => void;
  bootstrap: SessionBootstrapApi;
}

export interface SessionGreetingLifecycleApi {
  createBoundBridge: (
    localId: string,
    agentInstance: AgentInstance,
  ) => Promise<AcpClientBridge>;
  initializeConnectedBridge: (
    localId: string,
    bridge: AcpClientBridge,
    sessionState: SessionState,
  ) => Promise<InitializationResult>;
  handleLifecycleFailure: (
    localId: string,
    eventName: "SessionCreated" | "SessionResumed",
    agentName: string,
    error: unknown,
  ) => Promise<never>;
  finalizeConnectionAttempt: (localId: string) => void;
}

function createAgentInstance(name: string): AgentInstance {
  return {
    id: crypto.randomUUID(),
    name,
  };
}

export function useSessionGreeting({
  sessions,
  bridges,
  savedSessions,
  saveSessionsToStore,
  mergeSessionsFromAgent,
  handleSessionUpdate,
  clearConnectionError,
  bindPendingPermission,
  stopPermissionWatch,
  createSessionSlot,
  clearSessionAbort,
  bootstrap,
}: UseSessionGreetingDeps) {
  async function createBoundBridge(
    localId: string,
    agentInstance: AgentInstance,
  ): Promise<AcpClientBridge> {
    let bridge: AcpClientBridge;
    const writePolicy = createWritePermissionPolicy(
      localId,
      sessions,
      () => bridge,
    );
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
            role: "system",
            content: message.message,
            timestamp: Date.now(),
          });
        }
      },
      writePolicy,
    });
    bridges.set(localId, bridge);
    bridge.setPermissionModeProvider(
      () => sessions.value.get(localId)?.currentModeId ?? null,
    );
    bindPendingPermission(localId, bridge);
    bridge.onSessionUpdate = (notification) =>
      handleSessionUpdate(localId, notification);
    await bootstrap.ensureNotAborted(localId, () => bridge.disconnect());
    return bridge;
  }

  async function initializeConnectedBridge(
    localId: string,
    bridge: AcpClientBridge,
    sessionState: SessionState,
  ) {
    const initialized = await bootstrap.performInitialization(bridge);
    sessionState.agentInfo = initialized.agentInfoResult;
    sessionState.agentPromptCapabilities = initialized.promptCaps;
    sessionState.supportsSessionList = Boolean(
      initialized.initResponse.agentCapabilities?.sessionCapabilities?.list,
    );
    await bootstrap.ensureNotAborted(localId, () => bridge.disconnect());
    return initialized;
  }

  async function handleLifecycleFailure(
    localId: string,
    eventName: "SessionCreated" | "SessionResumed",
    agentName: string,
    error: unknown,
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
        const cleanupError =
          disconnectError instanceof Error
            ? disconnectError
            : new Error(String(disconnectError));
        if (session) {
          session.error = `${session.error ?? "Connection failed"}; cleanup failed: ${cleanupError.message}`;
        }
        trackError(cleanupError);
      }
    }
    trackEvent(eventName, { agentName, success: "false" });
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
      startupPhase: "starting",
      source: options.source ?? "agents",
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
      sessionState.startupPhase = "initializing";

      const bridge = await createBoundBridge(localId, agentInstance);
      sessionState.startupPhase = "connecting";

      const initialized = await initializeConnectedBridge(
        localId,
        bridge,
        sessionState,
      );
      if (import.meta.env.DEV) {
      }

      const startupConfigOptions =
        Object.keys(configOptions).length > 0 ? { configOptions } : {};
      const sessionResponse = await bootstrap.runWithAuthenticationRetry(
        localId,
        bridge,
        sessionState,
        agentName,
        initialized.availableAuthMethods,
        () =>
          bridge.newSession({ cwd, mcpServers: [], ...startupConfigOptions }),
        "Authentication cancelled by user",
      );

      sessionState.session = {
        id: crypto.randomUUID(),
        agentName,
        sessionId: sessionResponse.sessionId,
        title: createDefaultSessionTitle(agentName, savedSessions.value),
        lastUpdated: Date.now(),
        cwd,
        supportsLoadSession:
          initialized.initResponse.agentCapabilities?.loadSession ?? false,
      };
      sessionState.isConnected = true;
      sessionState.messages = [];
      sessionState.toolCalls.clear();
      savedSessions.value.push(sessionState.session);
      await saveSessionsToStore();

      if (sessionState.supportsSessionList) {
        await mergeSessionsFromAgent(bridge, agentName, cwd);
      }

      trackEvent("SessionCreated", { agentName, success: "true" });
      applySessionConfigurationState(sessionState, sessionResponse);
      clearConnectionError();
    } catch (error) {
      await handleLifecycleFailure(localId, "SessionCreated", agentName, error);
    } finally {
      finalizeConnectionAttempt(localId);
      if (startupTimer) {
        clearInterval(startupTimer);
      }
    }
  }

  return {
    createSession,
    createBoundBridge,
    initializeConnectedBridge,
    handleLifecycleFailure,
    finalizeConnectionAttempt,
  };
}
