// ACP session-update replay reducer (T-600.7).
//
// A pure, framework-free port of the session store's `handleSessionUpdate`
// switch (stores/session.ts:133-270) that folds `session/update` notifications
// into a renderable message list. The live SessionInspectorPanel reuses THIS so
// the pipeline-node observer renders agent_message_chunk / user_message_chunk /
// agent_thought_chunk / tool_call / tool_call_update exactly like the Agents tab
// chat — read-only: we OBSERVE the stream, we never send a prompt.
//
// Keeping it standalone (rather than calling into the session store, which closes
// over its own private refs) lets the inspection store buffer messages per-node
// and lets blue-team unit-test the fold without a live transport.

import type { SessionNotification } from '@agentclientprotocol/sdk';
import type { ChatMessage, ToolCallInfo } from './types';

export interface ReplayState {
  messages: ChatMessage[];
  toolCalls: Map<string, ToolCallInfo>;
}

// Cap the live replay buffer so a long-running observed session cannot grow the
// message array without bound. Chunks coalesce into the trailing message, so a
// single turn is one entry; this caps DISTINCT messages, keeping the most recent.
export const MAX_REPLAY_MESSAGES = 1000;

function capMessages(state: ReplayState): void {
  const overflow = state.messages.length - MAX_REPLAY_MESSAGES;
  if (overflow > 0) state.messages.splice(0, overflow);
}

export function createReplayState(): ReplayState {
  return { messages: [], toolCalls: new Map() };
}

function newId(): string {
  // crypto.randomUUID is available in the browser, Tauri webview, and the Node
  // ≥18 test runtime (vitest). Mirrors session.ts's id generation.
  return crypto.randomUUID();
}

/**
 * Fold a single `session/update` notification into the replay state, mutating
 * `state.messages` / `state.toolCalls` in place. Mirrors the session store's
 * handler case-for-case for the five message-relevant variants; non-message
 * variants (config/mode/commands) are intentionally ignored by the observer.
 */
export function applySessionUpdate(
  state: ReplayState,
  notification: SessionNotification
): void {
  const update = notification.update;

  switch (update.sessionUpdate) {
    case 'user_message_chunk': {
      const last = state.messages[state.messages.length - 1];
      if (last && last.role === 'user') {
        if (update.content.type === 'text') last.content += update.content.text;
      } else {
        state.messages.push({
          id: newId(),
          role: 'user',
          content: update.content.type === 'text' ? update.content.text : '',
          timestamp: Date.now(),
        });
      }
      break;
    }

    case 'agent_message_chunk': {
      const last = state.messages[state.messages.length - 1];
      if (last && last.role === 'assistant') {
        if (update.content.type === 'text') last.content += update.content.text;
      } else {
        state.messages.push({
          id: newId(),
          role: 'assistant',
          content: update.content.type === 'text' ? update.content.text : '',
          timestamp: Date.now(),
          toolCalls: [],
        });
      }
      break;
    }

    case 'agent_thought_chunk': {
      const last = state.messages[state.messages.length - 1];
      if (last && last.role === 'assistant') {
        if (update.content.type === 'text') {
          last.thought = (last.thought ?? '') + update.content.text;
        }
      } else {
        state.messages.push({
          id: newId(),
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
      const info: ToolCallInfo = {
        toolCallId: update.toolCallId,
        title: update.title,
        kind: update.kind ?? 'other',
        status: update.status ?? 'pending',
        locations: update.locations ?? undefined,
      };
      const current = state.messages[state.messages.length - 1];
      if (current && current.role === 'assistant') {
        if (!current.toolCalls) current.toolCalls = [];
        current.toolCalls.push({ ...info });
      }
      state.toolCalls.set(update.toolCallId, info);
      break;
    }

    case 'tool_call_update': {
      const existing = state.toolCalls.get(update.toolCallId);
      if (existing) {
        if (update.status) existing.status = update.status;
        if (update.title) existing.title = update.title;
        for (const msg of state.messages) {
          if (!msg.toolCalls) continue;
          const tc = msg.toolCalls.find((t) => t.toolCallId === update.toolCallId);
          if (tc) {
            if (update.status) tc.status = update.status;
            if (update.title) tc.title = update.title;
          }
        }
      }
      break;
    }

    default:
      // Non-message variants (current_mode_update, config_option_update,
      // available_commands_update, …) carry no chat content for the observer.
      break;
  }

  capMessages(state);
}
