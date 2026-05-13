import type { Ref } from 'vue';
import type { ChatMessage, ContentBlock, McpAppUiMetadata, SavedSession, SessionState, AvailableCommand } from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { trackEvent } from '../lib/telemetry';
import { deriveSessionTitleFromFirstMessage, isGeneratedSessionTitle, parseUpdatedAt } from './session-utils';
import { applyConfigOptionsUpdate, getActiveSessionContext, requireActiveSessionContext } from './session-state-helpers';
import { cloneContentBlock, handleUserMessageChunk, handleAgentMessageChunk, handleAgentThoughtChunk, handleToolCall, handleToolCallUpdate } from './useStreamHandler';

interface UseMessageHandlerDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
  cancelPermission: () => void;
  resolveToolAppUi?: (toolName: string | null | undefined) => McpAppUiMetadata | null | undefined;
}

export function useMessageHandler({
  sessions,
  bridges,
  activeSessionId,
  savedSessions,
  saveSessionsToStore,
  cancelPermission,
  resolveToolAppUi,
}: UseMessageHandlerDeps) {
  function cloneMessageForPersistence(message: ChatMessage): ChatMessage {
    return {
      id: message.id,
      role: message.role,
      content: message.content,
      blocks: message.blocks?.map((block) => cloneContentBlock(block)),
      thought: message.thought,
      timestamp: message.timestamp,
      stopReason: message.stopReason,
      toolCalls: message.toolCalls?.map((toolCall) => ({
        ...toolCall,
        locations: toolCall.locations?.map((location) => ({ ...location })),
        content: toolCall.content?.map((block) => ({ ...block })),
        rawInput: toolCall.rawInput ? { ...toolCall.rawInput } : undefined,
        rawOutput: toolCall.rawOutput ? { ...toolCall.rawOutput } : undefined,
      })),
    };
  }

  async function persistSessionMessages(session: SessionState): Promise<void> {
    const sessionInfo = session.session;
    if (!sessionInfo) {
      return;
    }
    const savedSession = savedSessions.value.find((entry) => entry.sessionId === sessionInfo.sessionId);
    if (!savedSession) {
      return;
    }
    savedSession.messages = session.messages.map((message) => cloneMessageForPersistence(message));
    savedSession.lastUpdated = Date.now();
    await saveSessionsToStore();
  }

  function renameSessionFromFirstPrompt(session: SessionState, text: string): void {
    if (!session.session || session.messages.some((message) => message.role === 'user')) {
      return;
    }

    if (!isGeneratedSessionTitle(session.session.title, session.session.agentName || session.agentName)) {
      return;
    }

    const nextTitle = deriveSessionTitleFromFirstMessage(text);
    if (!nextTitle) {
      return;
    }

    session.session.title = nextTitle;
    const savedSession = savedSessions.value.find((entry) => entry.sessionId === session.session?.sessionId);
    if (savedSession) {
      savedSession.title = nextTitle;
    }
  }

  function pushSystemMessage(session: SessionState, content: string): void {
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'system',
      content,
      timestamp: Date.now(),
    });
  }

  function cancelPendingToolCalls(session: SessionState): void {
    for (const toolCall of session.toolCalls.values()) {
      if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
        toolCall.status = 'cancelled';
      }
    }

    for (const message of session.messages) {
      if (!message.toolCalls) {
        continue;
      }

      for (const toolCall of message.toolCalls) {
        if (toolCall.status === 'pending' || toolCall.status === 'in_progress') {
          toolCall.status = 'cancelled';
        }
      }
    }
  }

  function handleSessionUpdate(localSessionId: string, notification: SessionNotification) {
    const session = sessions.value.get(localSessionId);
    if (!session) return;

    const update = notification.update;

    if ((update as { sessionUpdate?: string }).sessionUpdate === 'usage_update') {
      const { sessionUpdate: _ignored, ...usage } = update as Record<string, unknown> & {
        sessionUpdate: string;
      };
      session.usage = { ...usage };
      return;
    }

    switch (update.sessionUpdate) {
      case 'user_message_chunk':
        handleUserMessageChunk(session, update);
        break;

      case 'agent_message_chunk':
        handleAgentMessageChunk(session, update);
        break;

      case 'agent_thought_chunk':
        handleAgentThoughtChunk(session, update);
        break;

      case 'tool_call':
        handleToolCall(session, update, resolveToolAppUi?.(update.title ?? null) ?? null);
        break;

      case 'tool_call_update':
        handleToolCallUpdate(session, update, resolveToolAppUi?.(update.title ?? null) ?? null);
        break;

      case 'config_option_update':
        applyConfigOptionsUpdate(session, update.configOptions);
        break;

      case 'plan':
        if ('entries' in update && Array.isArray(update.entries)) {
          session.currentPlan = update.entries.map((entry) => ({
            content: entry.content,
            priority: entry.priority ?? 'medium',
            status: entry.status ?? 'pending',
            _meta: entry._meta ?? null,
          }));
        }
        break;

      case 'available_commands_update':
        if ('availableCommands' in update && Array.isArray(update.availableCommands)) {
          session.availableCommands = update.availableCommands as AvailableCommand[];
        }
        break;

      case 'current_mode_update':
        session.currentModeId = update.currentModeId;
        break;

      case 'session_info_update': {
        const targetSessionId = notification.sessionId;
        const nextTitle =
          'title' in update
            ? update.title === null
              ? ''
              : update.title ?? undefined
            : undefined;
        const nextLastUpdated =
          'updatedAt' in update && update.updatedAt !== undefined
            ? update.updatedAt === null
              ? 0
              : parseUpdatedAt(update.updatedAt)
            : undefined;

        let savedSessionModified = false;
        const savedSession = savedSessions.value.find(
          (s) => s.sessionId === targetSessionId,
        );
        if (savedSession) {
          if (nextTitle !== undefined) {
            savedSession.title = nextTitle;
            savedSessionModified = true;
          }
          if (nextLastUpdated !== undefined) {
            savedSession.lastUpdated = nextLastUpdated;
            savedSessionModified = true;
          }
        }

        if (session.session?.sessionId === targetSessionId) {
          if (nextTitle !== undefined) {
            session.session.title = nextTitle;
          }
          if (nextLastUpdated !== undefined) {
            session.session.lastUpdated = nextLastUpdated;
          }
        }

        if (savedSessionModified) {
          saveSessionsToStore();
        }
        break;
      }

      default:
        trackEvent('UnknownAcpSessionUpdate', {
          sessionUpdate: String((update as Record<string, unknown>).sessionUpdate ?? 'unknown'),
        });
    }
  }

  async function sendPrompt(text: string, attachments: ContentBlock[] = []): Promise<boolean> {
    let activeSession: SessionState | null = null;
    try {
      const { session, bridge } = requireActiveSessionContext(activeSessionId, sessions, bridges);
      activeSession = session;
      renameSessionFromFirstPrompt(session, text);

      const prompt = [
        { type: 'text', text } as const,
        ...attachments.map((attachment) => cloneContentBlock(attachment)),
      ];

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
        const hasVisibleOutput = lastMsg?.role === 'assistant' && (
          Boolean(lastMsg.content?.trim()) ||
          Boolean(lastMsg.blocks?.length)
        );
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
        pushSystemMessage(activeSession, errorMessage);
      }
      return false;
    }
  }

  async function cancelOperation(): Promise<void> {
    const context = getActiveSessionContext(activeSessionId, sessions, bridges);
    if (!context) return;

    const { session, bridge } = context;

    if (session.pendingPermissions.length > 0 || bridge.pendingPermissionRequests.value.length > 0) {
      cancelPermission();
    }

    cancelPendingToolCalls(session);

    await bridge.cancel({
      sessionId: session.session.sessionId,
    });
  }

  return {
    handleSessionUpdate,
    sendPrompt,
    cancelOperation,
    pushSystemMessage,
    cancelPendingToolCalls,
  };
}
