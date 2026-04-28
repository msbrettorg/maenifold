// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { reactive, ref } from 'vue';
import { createEmptySessionState } from '../../lib/types';
import type { SavedSession } from '../../lib/types';
import { useMessageHandler } from '../useMessageHandler';

describe('useMessageHandler', () => {
  it('stores usage_update payloads on session state', () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    const sessions = ref(new Map([[session.sessionId, session]]));
    const handler = useMessageHandler({
      sessions,
      bridges: new Map(),
      activeSessionId: ref(session.sessionId),
      savedSessions: ref<SavedSession[]>([]),
      saveSessionsToStore: async () => {},
      cancelPermission: () => {},
    });

    handler.handleSessionUpdate(session.sessionId, {
      sessionId: 'remote-1',
      update: {
        sessionUpdate: 'usage_update',
        inputTokens: 42,
        outputTokens: 7,
        cacheWriteTokens: 3,
      },
    } as never);

    expect(session.usage).toEqual({
      inputTokens: 42,
      outputTokens: 7,
      cacheWriteTokens: 3,
    });
  });

  it('updates the current session mode from current_mode_update notifications', () => {
    const session = reactive(createEmptySessionState('local-1', 'OpenCode', '/repo'));
    const sessions = ref(new Map([[session.sessionId, session]]));
    const handler = useMessageHandler({
      sessions,
      bridges: new Map(),
      activeSessionId: ref(session.sessionId),
      savedSessions: ref<SavedSession[]>([]),
      saveSessionsToStore: async () => {},
      cancelPermission: () => {},
    });

    handler.handleSessionUpdate(session.sessionId, {
      sessionId: 'remote-1',
      update: {
        sessionUpdate: 'current_mode_update',
        currentModeId: 'architect',
      },
    } as never);

    expect(session.currentModeId).toBe('architect');
  });

  it('renames a new session from the first user message', async () => {
    const session = reactive(createEmptySessionState('local-1', 'Architect', '/repo'));
    session.session = {
      id: 'saved-1',
      agentName: 'Architect',
      sessionId: 'remote-1',
      title: 'Architect Session #1',
      lastUpdated: Date.now(),
      cwd: '/repo',
    };

    const saveSessionsToStore = vi.fn(async () => {});
    const sessions = ref(new Map([[session.sessionId, session]]));
    const savedSessions = ref<SavedSession[]>([{ ...session.session }]);
    const handler = useMessageHandler({
      sessions,
      bridges: new Map([
        [
          session.sessionId,
          {
            prompt: vi.fn(async () => ({ stopReason: 'end_turn' })),
          } as never,
        ],
      ]),
      activeSessionId: ref(session.sessionId),
      savedSessions,
      saveSessionsToStore,
      cancelPermission: () => {},
    });

    await handler.sendPrompt('1234567890123456789012345678901234567890EXTRA');

    expect(session.session.title).toBe('1234567890123456789012345678901234567890');
    expect(savedSessions.value[0]?.title).toBe('1234567890123456789012345678901234567890');
    expect(saveSessionsToStore).toHaveBeenCalledTimes(1);
  });

  it('does not rename sessions that already have a meaningful title', async () => {
    const session = reactive(createEmptySessionState('local-1', 'Architect', '/repo'));
    session.session = {
      id: 'saved-1',
      agentName: 'Architect',
      sessionId: 'remote-1',
      title: 'Bug triage session',
      lastUpdated: Date.now(),
      cwd: '/repo',
    };

    const saveSessionsToStore = vi.fn(async () => {});
    const sessions = ref(new Map([[session.sessionId, session]]));
    const handler = useMessageHandler({
      sessions,
      bridges: new Map([
        [
          session.sessionId,
          {
            prompt: vi.fn(async () => ({ stopReason: 'end_turn' })),
          } as never,
        ],
      ]),
      activeSessionId: ref(session.sessionId),
      savedSessions: ref<SavedSession[]>([{ ...session.session }]),
      saveSessionsToStore,
      cancelPermission: () => {},
    });

    await handler.sendPrompt('A first visible prompt should not replace the saved title');

    expect(session.session.title).toBe('Bug triage session');
    expect(saveSessionsToStore).not.toHaveBeenCalled();
  });
});
