import { describe, expect, it, vi } from 'vitest';
import { reactive, ref } from 'vue';
import { useMessageHandler } from '../../composables/useMessageHandler';
import { createEmptySessionState, type SavedSession, type SessionState } from '../../lib/types';
import type { SessionNotification } from '@agentclientprotocol/sdk';

function createHarness() {
  const localSessionId = 'local-session';
  const session = reactive(createEmptySessionState(localSessionId, 'agent', '/repo')) as SessionState;
  session.session = {
    id: localSessionId,
    agentName: 'agent',
    sessionId: 'remote-session',
    title: 'Agent session',
    lastUpdated: Date.now(),
    cwd: '/repo',
  };
  const sessions = ref(new Map<string, SessionState>([[localSessionId, session]]));
  const savedSessions = ref<SavedSession[]>([]);
  const handler = useMessageHandler({
    sessions,
    bridges: new Map(),
    activeSessionId: ref(localSessionId),
    savedSessions,
    saveSessionsToStore: vi.fn(),
    cancelPermission: vi.fn(),
  });

  return { localSessionId, session, handler };
}

function notification(update: Record<string, unknown>): SessionNotification {
  return {
    sessionId: 'remote-session',
    update,
  } as unknown as SessionNotification;
}

describe('acp-ui session update dispatcher', () => {
  it('maps every acp-ui sessionUpdate kind into the active session state', () => {
    const { localSessionId, session, handler } = createHarness();
    const updates = [
      notification({ sessionUpdate: 'user_message_chunk', content: { type: 'text', text: 'hello' } }),
      notification({ sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: 'world' } }),
      notification({ sessionUpdate: 'agent_thought_chunk', content: { type: 'text', text: 'thinking' } }),
      notification({
        sessionUpdate: 'tool_call',
        toolCallId: 'tool-1',
        title: 'Read file',
        kind: 'read',
        status: 'pending',
        locations: [{ path: '/repo/README.md' }],
      }),
      notification({
        sessionUpdate: 'tool_call_update',
        toolCallId: 'tool-1',
        title: 'Read file done',
        status: 'completed',
      }),
      notification({
        sessionUpdate: 'available_commands_update',
        availableCommands: [{ name: 'help', description: 'Show help' }],
      }),
    ];

    for (const item of updates) {
      handler.handleSessionUpdate(localSessionId, item);
    }

    expect(session.messages[0]?.role).toBe('user');
    expect(session.messages[0]?.content).toBe('hello');
    expect(session.messages[1]?.role).toBe('assistant');
    expect(session.messages[1]?.content).toBe('world');
    expect(session.messages[1]?.thought).toBe('thinking');
    expect(session.messages[1]?.toolCalls?.[0]).toMatchObject({
      toolCallId: 'tool-1',
      title: 'Read file done',
      kind: 'read',
      status: 'completed',
      locations: [{ path: '/repo/README.md' }],
    });
    expect(session.toolCalls.get('tool-1')).toMatchObject({
      title: 'Read file done',
      status: 'completed',
    });
    expect(session.availableCommands[0]?.name).toBe('help');
  });

  it('mutates both the tool-call map entry and per-message toolCalls entry', () => {
    const { localSessionId, session, handler } = createHarness();

    handler.handleSessionUpdate(localSessionId, notification({
      sessionUpdate: 'agent_message_chunk',
      content: { type: 'text', text: 'I will inspect it.' },
    }));
    handler.handleSessionUpdate(localSessionId, notification({
      sessionUpdate: 'tool_call',
      toolCallId: 'tool-2',
      title: 'Edit file',
      kind: 'edit',
      status: 'in_progress',
      locations: [{ path: '/repo/src/main.ts' }],
    }));

    const mapEntry = session.toolCalls.get('tool-2');
    const messageEntry = session.messages[0]?.toolCalls?.[0];
    expect(mapEntry).toBeTruthy();
    expect(messageEntry).toBeTruthy();

    handler.handleSessionUpdate(localSessionId, notification({
      sessionUpdate: 'tool_call_update',
      toolCallId: 'tool-2',
      title: 'Edit file complete',
      status: 'completed',
      content: [{ type: 'content', content: { type: 'text', text: 'done' } }],
    }));

    expect(session.toolCalls.get('tool-2')).toBe(mapEntry);
    expect(session.messages[0]?.toolCalls?.[0]).toBe(messageEntry);
    expect(mapEntry).toMatchObject({ title: 'Edit file complete', status: 'completed' });
    expect(messageEntry).toMatchObject({ title: 'Edit file complete', status: 'completed' });
    expect(mapEntry?.content?.[0]).toMatchObject({ type: 'content' });
    expect(messageEntry?.content?.[0]).toMatchObject({ type: 'content' });
  });
});
