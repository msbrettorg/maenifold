// Smoke tests for useMessageHandler composable
import { describe, it, expect, vi } from 'vitest';
import { ref, reactive } from 'vue';
import { createPinia, setActivePinia } from 'pinia';
import { createEmptySessionState } from '../../lib/types';
import { useMessageHandler } from '../useMessageHandler';
import type { SessionNotification } from '@agentclientprotocol/sdk';

/** Scaffold a handler with a single reactive session. */
function createHandler() {
  // Pinia must be active because the AcpClientBridge constructor
  // (transitively imported as a type) lazily touches the traffic store.
  setActivePinia(createPinia());

  const localId = 'test-session-1';
  const session = reactive(createEmptySessionState(localId, 'test-agent', '/test'));
  const sessions = ref(new Map([[localId, session]]));
  const bridges = new Map();
  const activeSessionId = ref<string | null>(localId);
  const savedSessions = ref<any[]>([]);
  const saveSessionsToStore = vi.fn().mockResolvedValue(undefined);
  const cancelPermission = vi.fn();

  const handler = useMessageHandler({
    sessions: sessions as any,
    bridges,
    activeSessionId,
    savedSessions,
    saveSessionsToStore,
    cancelPermission,
  });

  return { handler, session, sessions, localId };
}

describe('useMessageHandler', () => {
  it('agent_message_chunk appends content to active session message', () => {
    const { handler, session, localId } = createHandler();

    // First chunk creates a new assistant message
    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'agent_message_chunk',
        content: { type: 'text', text: 'Hello ' },
      },
    } as SessionNotification);

    expect(session.messages).toHaveLength(1);
    expect(session.messages[0].role).toBe('assistant');
    expect(session.messages[0].content).toBe('Hello ');

    // Second chunk appends to the existing assistant message
    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'agent_message_chunk',
        content: { type: 'text', text: 'World!' },
      },
    } as SessionNotification);

    expect(session.messages).toHaveLength(1);
    expect(session.messages[0].content).toBe('Hello World!');
  });

  it('tool_call creates stub assistant message for tool-only turns', () => {
    const { handler, session, localId } = createHandler();

    // Start with a user message to simulate a real prompt turn
    session.messages.push({
      id: 'user-msg-1',
      role: 'user',
      content: 'Do something',
      timestamp: Date.now(),
    });

    // Agent emits a tool_call before any assistant text
    handler.handleSessionUpdate(localId, {
      sessionId: 'server-session-1',
      update: {
        sessionUpdate: 'tool_call',
        toolCallId: 'tc-1',
        title: 'Read file',
        kind: 'read',
        status: 'pending',
      },
    } as SessionNotification);

    // Should have created a stub assistant message with the tool call
    expect(session.messages).toHaveLength(2);
    const assistantMsg = session.messages[1];
    expect(assistantMsg.role).toBe('assistant');
    expect(assistantMsg.content).toBe('');
    expect(assistantMsg.toolCalls).toHaveLength(1);
    expect(assistantMsg.toolCalls![0].toolCallId).toBe('tc-1');
    expect(assistantMsg.toolCalls![0].title).toBe('Read file');
  });
});
