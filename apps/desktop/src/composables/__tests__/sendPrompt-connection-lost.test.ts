import { describe, expect, it, vi } from 'vitest';
import { reactive, ref } from 'vue';

import { createEmptySessionState } from '../../lib/types';
import type { SavedSession } from '../../lib/types';
import { useMessageHandler } from '../useMessageHandler';

function createHarness(options: {
  transportClosed: boolean;
  promptRejectsWith?: Error;
  onPrompt?: () => void;
}) {
  const session = reactive(createEmptySessionState('local-1', 'Architect', '/repo'));
  session.session = {
    id: 'saved-1',
    agentName: 'Architect',
    sessionId: 'remote-1',
    title: 'Architect Session #1',
    lastUpdated: Date.now(),
    cwd: '/repo',
  };
  session.isConnected = true;
  const bridge = {
    prompt: vi.fn(async () => {
      options.onPrompt?.();
      if (options.promptRejectsWith) throw options.promptRejectsWith;
      throw new Error('prompt failed');
    }),
    isTransportClosed: vi.fn(() => options.transportClosed),
  };
  const handler = useMessageHandler({
    sessions: ref(new Map([[session.sessionId, session]])),
    bridges: new Map([[session.sessionId, bridge as never]]),
    activeSessionId: ref(session.sessionId),
    savedSessions: ref<SavedSession[]>([{ ...session.session }]),
    saveSessionsToStore: vi.fn(async () => {}),
    cancelPermission: () => {},
  });
  return { handler, session, bridge };
}

describe('sendPrompt connection_lost handling', () => {
  it('marks the streaming assistant message and suppresses a system message on transport close', async () => {
    const { handler, session } = createHarness({
      transportClosed: true,
      promptRejectsWith: new Error('ACP agent terminated before responding'),
      onPrompt: () => {
        session.messages.push({
          id: 'assistant-1',
          role: 'assistant',
          content: 'partial',
          timestamp: Date.now(),
        });
      },
    });

    const success = await handler.sendPrompt('continue');

    expect(success).toBe(false);
    expect(session.isConnected).toBe(false);
    expect(session.messages.find((message) => message.id === 'assistant-1')?.stopReason).toBe('connection_lost');
    expect(session.messages.some((message) => message.role === 'system')).toBe(false);
  });

  it('inserts a placeholder assistant message when no assistant frame arrived', async () => {
    const { handler, session } = createHarness({
      transportClosed: true,
      promptRejectsWith: new Error('ACP agent terminated before responding'),
    });

    const success = await handler.sendPrompt('hello');
    const placeholder = session.messages[session.messages.length - 1];

    expect(success).toBe(false);
    expect(session.isConnected).toBe(false);
    expect(placeholder.role).toBe('assistant');
    expect(placeholder.content).toBe('');
    expect(placeholder.stopReason).toBe('connection_lost');
  });

  it('keeps the existing system-message behavior for non-transport prompt failures', async () => {
    const { handler, session } = createHarness({
      transportClosed: false,
      promptRejectsWith: new Error('agent rejected prompt'),
    });

    const success = await handler.sendPrompt('hello');

    expect(success).toBe(false);
    expect(session.isConnected).toBe(true);
    expect(session.messages.some((message) => message.stopReason === 'connection_lost')).toBe(false);
    const lastMessage = session.messages[session.messages.length - 1];
    expect(lastMessage.role).toBe('system');
    expect(lastMessage.content).toBe('agent rejected prompt');
  });
});
