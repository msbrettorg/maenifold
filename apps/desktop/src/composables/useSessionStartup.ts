// Session startup composable — create and resume flows built on shared bootstrap helpers.
import type { AcpClientBridge } from "../lib/acp-client";
import type { SessionSource, SessionState } from "../lib/types";
import type { AuthMethod } from "@agentclientprotocol/sdk";
import type { SessionRuntimeDeps } from "./session-runtime-deps";
import { useSessionBootstrap } from "./useSessionBootstrap";
import { useSessionGreeting } from "./useSessionGreeting";
import { useSessionRestore } from "./useSessionRestore";

interface UseSessionStartupDeps extends SessionRuntimeDeps {
  bindPendingPermission: (sessionId: string, bridge: AcpClientBridge) => void;
  stopPermissionWatch: (sessionId: string) => void;
  createSessionSlot: (
    agentName: string,
    cwd: string,
    options?: { startupPhase?: string; source?: SessionSource },
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
  clearConnectionError,
  bindPendingPermission,
  stopPermissionWatch,
  createSessionSlot,
  clearSessionAbort,
  isSessionAborted,
}: UseSessionStartupDeps) {
  const bootstrap = useSessionBootstrap({
    getAppVersion,
    promptForAuthMethod,
    isSessionAborted,
  });
  const greeting = useSessionGreeting({
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
    bootstrap,
  });
  const restore = useSessionRestore({
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
    createSessionSlot: (
      agentName: string,
      cwd: string,
      options?: { startupPhase?: string; source?: SessionSource },
    ) => createSessionSlot(agentName, cwd, options),
    bootstrap: {
      ensureNotAborted: bootstrap.ensureNotAborted,
      validateSavedSessionCwd: bootstrap.validateSavedSessionCwd,
      runWithAuthenticationRetry: <T>(
        localId: string,
        bridge: AcpClientBridge,
        sessionState: SessionState,
        agentName: string,
        authMethods: AuthMethod[],
        operation: () => Promise<T>,
        cancelledMessage: string | ((aborted: boolean) => string),
      ) =>
        bootstrap.runWithAuthenticationRetry(
          localId,
          bridge,
          sessionState,
          agentName,
          authMethods,
          operation,
          cancelledMessage,
        ),
    },
    lifecycle: {
      createBoundBridge: greeting.createBoundBridge,
      initializeConnectedBridge: greeting.initializeConnectedBridge,
      handleLifecycleFailure: greeting.handleLifecycleFailure,
      finalizeConnectionAttempt: greeting.finalizeConnectionAttempt,
    },
  });

  return {
    performInitialization: bootstrap.performInitialization,
    createSession: greeting.createSession,
    resumeSession: restore.resumeSession,
  };
}
