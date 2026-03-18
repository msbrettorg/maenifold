// Message handler composable — session update handling, prompt, and cancel
import type { Ref } from 'vue';
import type {
  SavedSession,
  SessionState,
  ToolCallInfo,
  AvailableCommand,
} from '../lib/types';
import type { AcpClientBridge } from '../lib/acp-bridge';
import type {
  SessionNotification,
  ToolCallContent as SdkToolCallContent,
  ToolCallLocation as SdkToolCallLocation,
} from '@agentclientprotocol/sdk';
import { trackEvent } from '../lib/telemetry';
import { normalizeConfigOptions, parseUpdatedAt } from './session-utils';

interface UseMessageHandlerDeps {
  sessions: Ref<Map<string, SessionState>>;
  bridges: Map<string, AcpClientBridge>;
  activeSessionId: Ref<string | null>;
  savedSessions: Ref<SavedSession[]>;
  saveSessionsToStore: () => Promise<void>;
  cancelPermission: () => void;
}

type ToolCallUpdateSnapshot = {
  toolCallId: string;
  title: string;
  kind?: string | null;
  status?: ToolCallInfo['status'] | null;
  locations?: Array<SdkToolCallLocation> | null;
  content?: Array<SdkToolCallContent> | null;
};

function normalizeToolCallLocations(
  locations: Array<SdkToolCallLocation> | null | undefined
): ToolCallInfo['locations'] | undefined {
  return locations?.map(({ path, line }) =>
    line == null ? { path } : { path, line }
  );
}

function normalizeToolCallContent(
  content: Array<SdkToolCallContent> | null | undefined
): ToolCallInfo['content'] | undefined {
  return content?.map((item) => ({ ...item }));
}

function createToolCallInfo(update: ToolCallUpdateSnapshot): ToolCallInfo {
  return {
    toolCallId: update.toolCallId,
    title: update.title,
    kind: update.kind ?? 'other',
    status: update.status ?? 'pending',
    locations: normalizeToolCallLocations(update.locations),
    content: normalizeToolCallContent(update.content),
  };
}

export function useMessageHandler({
  sessions,
  bridges,
  activeSessionId,
  savedSessions,
  saveSessionsToStore,
  cancelPermission,
}: UseMessageHandlerDeps) {
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
    notification: SessionNotification
  ) {
    const session = sessions.value.get(localSessionId);
    if (!session) return;

    const update = notification.update;

    switch (update.sessionUpdate) {
      case 'user_message_chunk': {
        // Append to last user message or create new (for replay)
        const lastUserMsg = session.messages[session.messages.length - 1];
        if (lastUserMsg && lastUserMsg.role === 'user') {
          if (update.content.type === 'text') {
            lastUserMsg.content += update.content.text;
          }
        } else {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'user',
            content: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
          });
        }
        break;
      }

      case 'agent_message_chunk': {
        // Append to last assistant message or create new
        const lastMsg = session.messages[session.messages.length - 1];
        if (lastMsg && lastMsg.role === 'assistant') {
          if (update.content.type === 'text') {
            lastMsg.content += update.content.text;
          }
        } else {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
            toolCalls: [],
          });
        }
        break;
      }

      case 'agent_thought_chunk': {
        // Append to last assistant message's thought field or create new
        const lastAssistantMsg = session.messages[session.messages.length - 1];
        if (lastAssistantMsg && lastAssistantMsg.role === 'assistant') {
          if (update.content.type === 'text') {
            lastAssistantMsg.thought = (lastAssistantMsg.thought || '') + update.content.text;
          }
        } else {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: '',
            thought: update.content.type === 'text' ? update.content.text : '',
            timestamp: Date.now(),
            toolCalls: [],
          });
        }
        break;
      }

      case 'tool_call': {
        const toolCallInfo = createToolCallInfo(update);

        // Add tool call to the current assistant message, synthesizing a
        // tool-only assistant turn when the agent emits tool activity before
        // any assistant text.
        let currentAssistantMsg = session.messages[session.messages.length - 1];
        if (!currentAssistantMsg || currentAssistantMsg.role !== 'assistant') {
          session.messages.push({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: '',
            timestamp: Date.now(),
            toolCalls: [],
          });
          currentAssistantMsg = session.messages[session.messages.length - 1];
        }

        if (!currentAssistantMsg.toolCalls) {
          currentAssistantMsg.toolCalls = [];
        }
        currentAssistantMsg.toolCalls.push({ ...toolCallInfo });

        // Also keep in session's tool call map for updates
        session.toolCalls.set(update.toolCallId, { ...toolCallInfo });
        break;
      }

      case 'tool_call_update': {
        const existing = session.toolCalls.get(update.toolCallId);
        if (existing) {
          if (update.status) existing.status = update.status;
          if (update.title) existing.title = update.title;
          const nextContent = normalizeToolCallContent(update.content);
          if (nextContent?.length) {
            existing.content = [...(existing.content ?? []), ...nextContent];
          }
          if (update.locations) {
            existing.locations = normalizeToolCallLocations(update.locations);
          }
          // Also update in the message's toolCalls array
          for (const msg of session.messages) {
            if (msg.toolCalls) {
              const tc = msg.toolCalls.find(t => t.toolCallId === update.toolCallId);
              if (tc) {
                if (update.status) tc.status = update.status;
                if (update.title) tc.title = update.title;
                if (nextContent?.length) {
                  tc.content = [...(tc.content ?? []), ...nextContent];
                }
                if (update.locations) {
                  tc.locations = normalizeToolCallLocations(update.locations);
                }
              }
            }
          }
        }
        break;
      }

      case 'current_mode_update':
        // Agent changed the mode
        if (session.hasConfigOptions) {
          break;
        }
        if ('modeId' in update && update.modeId) {
          session.currentModeId = update.modeId as string;
        }
        break;

      case 'config_option_update':
        session.configOptions = normalizeConfigOptions(update.configOptions);
        session.hasConfigOptions = true;
        session.availableModes = [];
        session.currentModeId = '';
        session.availableModels = [];
        session.currentModelId = '';
        break;

      case 'available_commands_update':
        // Agent advertised slash commands
        if ('availableCommands' in update && Array.isArray(update.availableCommands)) {
          session.availableCommands = update.availableCommands as AvailableCommand[];
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
          (s) => s.sessionId === targetSessionId
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
        console.log('Unhandled session update:', update);
    }
  }

  async function sendPrompt(text: string): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) throw new Error('No active session');
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) {
      throw new Error('No active session');
    }

    // Add user message
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    });

    session.isLoading = true;
    try {
      const response = await bridge.prompt({
        sessionId: session.session.sessionId,
        prompt: [
          {
            type: 'text',
            text,
          },
        ],
      });

      console.log('Prompt completed:', response.stopReason);

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
  }

  async function cancelOperation(): Promise<void> {
    const sid = activeSessionId.value;
    if (!sid) return;
    const session = sessions.value.get(sid);
    const bridge = bridges.get(sid);
    if (!session || !bridge || !session.session) return;

    if (session.pendingPermissions.length > 0 || bridge.pendingPermissionRequest.value) {
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
