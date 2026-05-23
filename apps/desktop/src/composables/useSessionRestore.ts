import type { AuthMethod } from "@agentclientprotocol/sdk";
import type { AcpClientBridge } from "../lib/acp-client";
import type { AgentInstance, SavedSession, SessionState } from "../lib/types";
import { trackError, trackEvent } from "../lib/telemetry";
import type { SessionRuntimeDeps } from "./session-runtime-deps";
import { applySessionConfigurationState } from "./session-utils";
import type { SessionGreetingLifecycleApi } from "./useSessionGreeting";

interface SessionRestoreBootstrapApi {
  ensureNotAborted: (
    sessionId: string,
    cleanup?: () => Promise<void> | void,
  ) => Promise<void>;
  validateSavedSessionCwd: (
    bridge: AcpClientBridge,
    savedSession: SavedSession,
  ) => Promise<void>;
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

interface UseSessionRestoreDeps extends SessionRuntimeDeps {
  createSessionSlot: (
    agentName: string,
    cwd: string,
  ) => { localId: string; sessionState: SessionState };
  bootstrap: SessionRestoreBootstrapApi;
  lifecycle: SessionGreetingLifecycleApi;
}

function isTransientResumeTransportError(
  error: unknown,
  bridge: AcpClientBridge,
): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return (
    bridge.isTransportClosed() ||
    message.includes("ACP connection closed") ||
    message.includes("ACP client disconnected before response") ||
    message.includes("ACP agent terminated before responding") ||
    message.includes("ACP relay socket is not connected")
  );
}

function waitForResumeRetry(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 1000));
}

function cloneSavedMessages(
  savedSession: SavedSession,
): SessionState["messages"] {
  return (
    savedSession.messages?.map((message) => ({
      ...message,
      blocks: message.blocks?.map((block) => ({ ...block })),
      toolCalls: message.toolCalls?.map((toolCall) => ({
        ...toolCall,
        locations: toolCall.locations?.map((location) => ({ ...location })),
        content: toolCall.content?.map((block) => ({ ...block })),
        rawInput: toolCall.rawInput ? { ...toolCall.rawInput } : undefined,
        rawOutput: toolCall.rawOutput ? { ...toolCall.rawOutput } : undefined,
      })),
    })) ?? []
  );
}

export function useSessionRestore({
  sessions,
  bridges,
  activeSessionId,
  saveSessionsToStore,
  clearConnectionError,
  createSessionSlot,
  bootstrap,
  lifecycle,
}: UseSessionRestoreDeps) {
  async function resumeSession(savedSession: SavedSession): Promise<void> {
    for (const [localId, session] of sessions.value.entries()) {
      if (
        session.session?.sessionId === savedSession.sessionId &&
        session.isConnected
      ) {
        activeSessionId.value = localId;
        return;
      }
    }

    const { localId, sessionState } = createSessionSlot(
      savedSession.agentName,
      savedSession.cwd,
    );

    try {
      let bridge: AcpClientBridge | null = null;
      let initialized: Awaited<
        ReturnType<SessionGreetingLifecycleApi["initializeConnectedBridge"]>
      > | null = null;
      let loadResponse: Awaited<
        ReturnType<AcpClientBridge["loadSession"]>
      > | null = null;
      let lastError: unknown = null;

      for (let attempt = 0; attempt < 2; attempt += 1) {
        const agentInstance: AgentInstance = {
          id: crypto.randomUUID(),
          name: savedSession.agentName,
        };
        sessionState.agentId = agentInstance.id;
        bridge = await lifecycle.createBoundBridge(localId, agentInstance);
        initialized = await lifecycle.initializeConnectedBridge(
          localId,
          bridge,
          sessionState,
        );

        if (!initialized.initResponse.agentCapabilities?.loadSession) {
          await bridge.disconnect();
          throw new Error("Agent no longer supports session loading");
        }

        sessionState.messages = [];
        sessionState.toolCalls.clear();

        try {
          loadResponse = await bootstrap.runWithAuthenticationRetry(
            localId,
            bridge,
            sessionState,
            savedSession.agentName,
            initialized.availableAuthMethods,
            async () => {
              await bootstrap.validateSavedSessionCwd(
                bridge as AcpClientBridge,
                savedSession,
              );
              return (bridge as AcpClientBridge).loadSession({
                sessionId: savedSession.sessionId,
                cwd: savedSession.cwd,
                mcpServers: [],
              });
            },
            (wasAborted) =>
              wasAborted
                ? "Connection cancelled"
                : "Authentication cancelled by user",
          );
          break;
        } catch (error) {
          lastError = error;
          const shouldRetry =
            attempt === 0 && isTransientResumeTransportError(error, bridge);
          bridges.delete(localId);
          await bridge.disconnect().catch((disconnectError) => {
            trackError(
              disconnectError instanceof Error
                ? disconnectError
                : new Error(String(disconnectError)),
            );
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
        throw (
          lastError ??
          new Error("Session resume failed before session/load completed")
        );
      }

      await bootstrap.ensureNotAborted(localId, () => bridge.disconnect());
      sessionState.session = savedSession;
      sessionState.isConnected = true;
      applySessionConfigurationState(sessionState, loadResponse);
      if (sessionState.messages.length === 0 && savedSession.messages?.length) {
        sessionState.messages = cloneSavedMessages(savedSession);
      }

      trackEvent("SessionResumed", {
        agentName: savedSession.agentName,
        success: "true",
      });
      savedSession.lastUpdated = Date.now();
      await saveSessionsToStore();
      clearConnectionError();
    } catch (error) {
      await lifecycle.handleLifecycleFailure(
        localId,
        "SessionResumed",
        savedSession.agentName,
        error,
      );
    } finally {
      lifecycle.finalizeConnectionAttempt(localId);
    }
  }

  return { resumeSession };
}
