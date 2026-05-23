import type { Ref } from 'vue';
import type { AvailableCommand, McpAppUiMetadata, SavedSession, SessionState } from '../lib/types';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { trackEvent } from '../lib/telemetry';
import { parseUpdatedAt } from './session-utils';
import { applyConfigOptionsUpdate } from './session-state-helpers';
import {
  handleAgentMessageChunk,
  handleAgentThoughtChunk,
  handleToolCall,
  handleToolCallUpdate,
  handleUserMessageChunk,
} from './useStreamHandler';

interface UseMessageHandlerSessionUpdateDeps {
  sessions: Ref<Map<string, SessionState>>;
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
  resolveToolAppUi?: (toolName: string | null | undefined) => McpAppUiMetadata | null | undefined;
}

export function handleSessionUpdate(
  localSessionId: string,
  notification: SessionNotification,
  { sessions, savedSessions, saveSessionsToStore, resolveToolAppUi }: UseMessageHandlerSessionUpdateDeps,
): void {
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
      const savedSession = savedSessions.value.find((candidate) => candidate.sessionId === targetSessionId);
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
        if (nextTitle !== undefined) session.session.title = nextTitle;
        if (nextLastUpdated !== undefined) session.session.lastUpdated = nextLastUpdated;
      }
      if (savedSessionModified) saveSessionsToStore();
      break;
    }
    default:
      trackEvent('UnknownAcpSessionUpdate', {
        sessionUpdate: String((update as Record<string, unknown>).sessionUpdate ?? 'unknown'),
      });
  }
}
