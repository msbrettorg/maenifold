import type { Ref } from 'vue';
import type { ContentBlock, SessionState } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import { trackEvent } from '../lib/telemetry';
import { getActiveSessionContext, requireActiveSessionContext } from './session-state-helpers';
import { cloneContentBlock } from './useStreamHandler';
import { markAssistantInterrupted, markLatestAssistantStopReason } from './useMessageHandler-session-core';

interface UseMessageHandlerPromptDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  persistSessionMessages: (session: SessionState) => Promise<void>;
  renameSessionFromFirstPrompt: (session: SessionState, text: string) => void;
  cancelPermission: () => void;
  pushSystemMessage: (session: SessionState, content: string) => void;
  cancelPendingToolCalls: (session: SessionState) => void;
}

export async function sendPrompt(
  text: string,
  attachments: ContentBlock[] = [],
  {
    sessions,
    bridges,
    activeSessionId,
    persistSessionMessages,
    renameSessionFromFirstPrompt,
    pushSystemMessage,
  }: UseMessageHandlerPromptDeps,
): Promise<boolean> {
  let activeSession: SessionState | null = null;
  let activeBridge: AcpClientBridge | null = null;
  try {
    const { session, bridge } = requireActiveSessionContext(activeSessionId, sessions, bridges);
    activeSession = session;
    activeBridge = bridge;
    renameSessionFromFirstPrompt(session, text);

    const prompt = [{ type: 'text', text } as const, ...attachments.map((attachment) => cloneContentBlock(attachment))];
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      blocks: prompt.map((block) => cloneContentBlock(block)),
      timestamp: Date.now(),
    });

    const assistantMessageStart = session.messages.length;
    session.isLoading = true;
    try {
      const response = await bridge.prompt({
        sessionId: session.session.sessionId,
        prompt,
      });

      const lastAssistantMessage = session.messages.slice(assistantMessageStart).findLast((message) => message.role === 'assistant');
      if (lastAssistantMessage) lastAssistantMessage.stopReason = response.stopReason ?? null;

      switch (response.stopReason) {
        case 'refusal':
          session.error = 'The agent refused this request.';
          break;
        case 'max_tokens':
          pushSystemMessage(session, 'Response was truncated due to token limits.');
          break;
        case 'max_turn_requests':
          pushSystemMessage(session, 'Turn limit reached.');
          break;
        case 'end_turn':
        case 'cancelled':
        default:
          break;
      }

      const lastMsg = session.messages[session.messages.length - 1];
      const hasVisibleOutput = lastMsg?.role === 'assistant' && (Boolean(lastMsg.content?.trim()) || Boolean(lastMsg.blocks?.length));
      if (response.stopReason === 'end_turn' && !hasVisibleOutput) {
        pushSystemMessage(
          session,
          'Agent completed but returned no visible output. This may indicate the agent does not stream content via ACP session updates.',
        );
      }

      trackEvent('PromptSent', {
        messageLength: String(text.length),
        stopReason: response.stopReason || 'unknown',
      });
    } finally {
      session.isLoading = false;
    }
    await persistSessionMessages(session);
    return true;
  } catch (err: unknown) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    if (activeSession) {
      activeSession.error = errorMessage;
      const isTransportClose = activeBridge?.isTransportClosed() ?? false;
      if (isTransportClose) {
        activeSession.isConnected = false;
        markAssistantInterrupted(activeSession);
      } else {
        pushSystemMessage(activeSession, errorMessage);
      }
    }
    return false;
  }
}

export async function cancelOperation({
  sessions,
  bridges,
  activeSessionId,
  persistSessionMessages,
  cancelPermission,
  cancelPendingToolCalls,
}: UseMessageHandlerPromptDeps): Promise<void> {
  const context = getActiveSessionContext(activeSessionId, sessions, bridges);
  if (!context) return;

  const { session, bridge } = context;
  if (session.pendingPermissions.length > 0 || bridge.pendingPermissionRequests.value.length > 0) {
    cancelPermission();
  }

  cancelPendingToolCalls(session);
  await bridge.cancel({ sessionId: session.session.sessionId });
  if (session.isLoading) {
    markLatestAssistantStopReason(session, 'cancelled');
    await persistSessionMessages(session);
  }
}
