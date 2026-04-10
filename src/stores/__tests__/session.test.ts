// Smoke test proving Map reassign triggers computed reactivity
import { describe, it, expect } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { reactive, nextTick } from 'vue';
import { useSessionStore } from '../session';
import { createEmptySessionState } from '../../lib/types';

describe('useSessionStore', () => {
  it('Map reassign triggers computed reactivity', async () => {
    setActivePinia(createPinia());
    const store = useSessionStore();

    // Initially no active session
    expect(store.hasActiveSession).toBe(false);

    // Build a session state and register it in the Map
    const localId = 'test-session';
    const session = reactive(
      createEmptySessionState(localId, 'test-agent', '/test')
    );
    session.session = {
      id: localId,
      agentName: 'test-agent',
      sessionId: 'server-session',
      title: 'Test Session',
      lastUpdated: Date.now(),
      cwd: '/test',
    };

    // Set into Map and reassign to trigger ref reactivity
    store.sessions.set(localId, session);
    store.sessions = new Map(store.sessions);
    store.activeSessionId = localId;

    await nextTick();

    // Verify computed properties reacted to the Map change
    expect(store.hasActiveSession).toBe(true);
    expect(store.currentSession).not.toBeNull();
    expect(store.currentSession!.title).toBe('Test Session');
  });
});
