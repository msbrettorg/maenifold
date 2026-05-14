import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { reactive } from 'vue';

import { createEmptySessionState } from '../../lib/types';
import { useSessionStore } from '../../stores/session';
import { useChatReconnect } from '../useChatReconnect';

describe('useChatReconnect', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('allows reconnect only for disconnected resumable sessions', () => {
    const store = useSessionStore();
    const session = reactive(createEmptySessionState('local-1', 'Architect', '/repo'));
    session.session = {
      id: 'saved-1',
      agentName: 'Architect',
      sessionId: 'remote-1',
      title: 'Architect Session #1',
      lastUpdated: Date.now(),
      cwd: '/repo',
      supportsLoadSession: true,
    };
    session.isConnected = false;
    session.isLoading = false;
    store.sessions.set(session.sessionId, session);
    store.sessions = new Map(store.sessions);
    store.activeSessionId = session.sessionId;

    const { canReconnect } = useChatReconnect();

    expect(canReconnect.value).toBe(true);
    session.isConnected = true;
    expect(canReconnect.value).toBe(false);
  });

  it('calls focusOrResumeRemoteSession with the active saved session', async () => {
    const store = useSessionStore();
    const session = reactive(createEmptySessionState('local-1', 'Architect', '/repo'));
    session.session = {
      id: 'saved-1',
      agentName: 'Architect',
      sessionId: 'remote-1',
      title: 'Architect Session #1',
      lastUpdated: Date.now(),
      cwd: '/repo',
      supportsLoadSession: true,
    };
    store.sessions.set(session.sessionId, session);
    store.sessions = new Map(store.sessions);
    store.activeSessionId = session.sessionId;
    const focusOrResume = vi.spyOn(store, 'focusOrResumeRemoteSession').mockResolvedValue({
      status: 'resumed',
      localSessionId: 'local-2',
    });

    const { reconnect } = useChatReconnect();
    await reconnect();

    expect(focusOrResume).toHaveBeenCalledWith(
      'remote-1',
      'Architect',
      { suppressErrors: false },
    );
  });
});
