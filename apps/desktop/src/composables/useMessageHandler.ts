// Message handler composable — session update handling, prompt, and cancel.
// Streaming chunk and tool-call processing is delegated to useStreamHandler.
import type { Ref } from 'vue';
import type {
  ContentBlock,
  McpAppUiMetadata,
  SavedSession,
  SessionState,
  AvailableCommand,
} from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-client';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { trackEvent } from '../lib/telemetry';
import {
  deriveSessionTitleFromFirstMessage,
  isClientCommand,
  isGeneratedSessionTitle,
  parseUpdatedAt,
} from './session-utils';
import {
  applyConfigOptionsUpdate,
  getActiveSessionContext,
  requireActiveSessionContext,
} from './session-state-helpers';
import {
  cloneContentBlock,
  handleUserMessageChunk,
  handleAgentMessageChunk,
  handleAgentThoughtChunk,
  handleToolCall,
  handleToolCallUpdate,
} from './useStreamHandler';

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
  async function renameSessionFromFirstPrompt(session: SessionState, text: string): Promise<void> {
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
    await saveSessionsToStore();
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

  function handleSessionUpdate(
    localSessionId: string,
    notification: SessionNotification,
  ) {
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
        // Agent advertised slash commands — merge with client-side commands
        if ('availableCommands' in update && Array.isArray(update.availableCommands)) {
          const clientCommands = session.availableCommands.filter(isClientCommand);
          session.availableCommands = [
            ...(update.availableCommands as AvailableCommand[]),
            ...clientCommands,
          ];
        }
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
        if (import.meta.env.DEV) {
        }
    }
  }

  async function sendPrompt(
    text: string,
    attachments: ContentBlock[] = [],
  ): Promise<boolean> {
    let activeSession: SessionState | null = null;
    try {
      const { session, bridge } = requireActiveSessionContext(activeSessionId, sessions, bridges);
      activeSession = session;
      await renameSessionFromFirstPrompt(session, text);

      const prompt = [
        { type: 'text', text } as const,
        ...attachments.map((attachment) => cloneContentBlock(attachment)),
      ];

      // Add user message
      session.messages.push({
        id: crypto.randomUUID(),
        role: 'user',
        content: text,
        blocks: prompt.map((block) => cloneContentBlock(block)),
        timestamp: Date.now(),
      });

      session.isLoading = true;
      try {
        const response = await bridge.prompt({
          sessionId: session.session.sessionId,
          prompt,
        });

        if (import.meta.env.DEV) {
        }

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

        // Track prompt sent
        trackEvent('PromptSent', {
          messageLength: String(text.length),
          stopReason: response.stopReason || 'unknown',
        });
      } finally {
        session.isLoading = false;
      }
      return true;
    } catch (err: unknown) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      if (import.meta.env.DEV) {
      }
      if (activeSession) {
        activeSession.error = errorMessage;
        pushSystemMessage(activeSession, errorMessage);
      }
      // Do NOT re-throw — the UI surfaces the error via session state and the
      // boolean return value lets callers decide whether to clear input.
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
