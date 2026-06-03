import { describe, expect, it } from 'vitest';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { applySessionUpdate, createReplayState, MAX_REPLAY_MESSAGES } from './acpReplay';

// Blue-team coverage (T-600.7): acpReplay is the pure fold that turns a stream of
// `session/update` notifications into the renderable transcript the live observer
// shows. It mirrors the session store's handler; these tests pin the five
// message-relevant cases so the read-only observer renders identically to the
// Agents tab.

function note(update: Record<string, unknown>): SessionNotification {
  return { sessionId: 's1', update } as unknown as SessionNotification;
}

function textChunk(kind: string, text: string): SessionNotification {
  return note({ sessionUpdate: kind, content: { type: 'text', text } });
}

describe('applySessionUpdate', () => {
  it('concatenates consecutive agent_message_chunks into one assistant message', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('agent_message_chunk', 'Hel'));
    applySessionUpdate(state, textChunk('agent_message_chunk', 'lo'));
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]).toMatchObject({ role: 'assistant', content: 'Hello' });
  });

  it('concatenates consecutive user_message_chunks into one user message', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('user_message_chunk', 'a'));
    applySessionUpdate(state, textChunk('user_message_chunk', 'b'));
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]).toMatchObject({ role: 'user', content: 'ab' });
  });

  it('starts a new message when the role changes', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('user_message_chunk', 'hi'));
    applySessionUpdate(state, textChunk('agent_message_chunk', 'yo'));
    expect(state.messages.map((m) => m.role)).toEqual(['user', 'assistant']);
  });

  it('caps the buffer at MAX_REPLAY_MESSAGES, evicting the oldest', () => {
    const state = createReplayState();
    // Alternate roles so every chunk becomes a NEW distinct message.
    for (let i = 0; i < MAX_REPLAY_MESSAGES + 100; i++) {
      const kind = i % 2 === 0 ? 'user_message_chunk' : 'agent_message_chunk';
      applySessionUpdate(state, textChunk(kind, `m${i}`));
    }
    expect(state.messages).toHaveLength(MAX_REPLAY_MESSAGES);
    // The oldest (m0..m99) were evicted; the newest is retained.
    expect(state.messages[state.messages.length - 1].content).toBe(
      `m${MAX_REPLAY_MESSAGES + 99}`
    );
    expect(state.messages[0].content).toBe('m100');
  });

  it('accumulates agent_thought_chunk onto the assistant message thought', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('agent_message_chunk', 'answer'));
    applySessionUpdate(state, textChunk('agent_thought_chunk', 'think'));
    applySessionUpdate(state, textChunk('agent_thought_chunk', 'ing'));
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]).toMatchObject({ content: 'answer', thought: 'thinking' });
  });

  it('opens a thought-only assistant message when a thought arrives first', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('agent_thought_chunk', 'pondering'));
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]).toMatchObject({ role: 'assistant', content: '', thought: 'pondering' });
  });

  it('attaches a tool_call to the current assistant message and tracks it', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('agent_message_chunk', 'calling'));
    applySessionUpdate(
      state,
      note({ sessionUpdate: 'tool_call', toolCallId: 'tc-1', title: 'read_file', kind: 'read', status: 'pending' })
    );
    expect(state.messages[0].toolCalls).toHaveLength(1);
    expect(state.messages[0].toolCalls?.[0]).toMatchObject({
      toolCallId: 'tc-1',
      title: 'read_file',
      status: 'pending',
    });
    expect(state.toolCalls.get('tc-1')?.status).toBe('pending');
  });

  it('applies tool_call_update to both the tracked call and the rendered copy', () => {
    const state = createReplayState();
    applySessionUpdate(state, textChunk('agent_message_chunk', 'calling'));
    applySessionUpdate(
      state,
      note({ sessionUpdate: 'tool_call', toolCallId: 'tc-1', title: 'read_file', status: 'pending' })
    );
    applySessionUpdate(
      state,
      note({ sessionUpdate: 'tool_call_update', toolCallId: 'tc-1', status: 'completed', title: 'read_file(done)' })
    );
    expect(state.toolCalls.get('tc-1')).toMatchObject({ status: 'completed', title: 'read_file(done)' });
    expect(state.messages[0].toolCalls?.[0]).toMatchObject({
      status: 'completed',
      title: 'read_file(done)',
    });
  });

  it('ignores non-message update variants without throwing', () => {
    const state = createReplayState();
    applySessionUpdate(state, note({ sessionUpdate: 'current_mode_update', currentModeId: 'x' }));
    applySessionUpdate(state, note({ sessionUpdate: 'available_commands_update', availableCommands: [] }));
    expect(state.messages).toHaveLength(0);
  });
});
