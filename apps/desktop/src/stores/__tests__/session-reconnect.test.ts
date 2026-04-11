import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { reactive } from 'vue';

import { createEmptySessionState } from '../../lib/types';
import { createMockBridge } from '../../test/setup';

const mockBridge = createMockBridge('resume-agent');
mockBridge.initialize.mockResolvedValue({
  protocolVersion: 1,
  authMethods: [],
  agentCapabilities: {
    loadSession: true,
    sessionCapabilities: {
      list: false,
    },
  },
});
mockBridge.loadSession.mockResolvedValue({});

vi.mock('../../lib/acp-client', () => ({
  createAcpClient: vi.fn(async () => mockBridge),
}));

import { useSessionStore } from '../session';

describe('useSessionStore reconnect flow', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockBridge.disconnect.mockClear();
    mockBridge.initialize.mockClear();
    mockBridge.loadSession.mockClear();
  });

  it('activates an existing live local session without creating a duplicate', async () => {
    const store = useSessionStore();
    const liveSession = reactive(createEmptySessionState('local-live', 'architect', '/repo'));
    liveSession.session = {
      id: 'saved-live',
      agentName: 'architect',
      sessionId: 'acp-remote-live',
      title: 'Existing session',
      lastUpdated: Date.now(),
      cwd: '/repo',
      supportsLoadSession: true,
    };
    liveSession.isConnected = true;

    store.sessions.set('local-live', liveSession);
    store.sessions = new Map(store.sessions);
    store.activeSessionId = null;

    const result = await store.focusOrResumeRemoteSession('acp-remote-live');

    expect(result).toEqual({ status: 'activated', localSessionId: 'local-live' });
    expect(store.activeSessionId).toBe('local-live');
    expect(store.sessions.size).toBe(1);
    expect(mockBridge.loadSession).not.toHaveBeenCalled();
  });

  it('prefers the newest connected local match over older disconnected duplicates', async () => {
    const store = useSessionStore();
    const staleSession = reactive(createEmptySessionState('local-stale', 'architect', '/repo'));
    staleSession.session = {
      id: 'saved-stale',
      agentName: 'architect',
      sessionId: 'acp-remote-live',
      title: 'Stale disconnected session',
      lastUpdated: 100,
      cwd: '/repo',
      supportsLoadSession: true,
    };

    const liveSession = reactive(createEmptySessionState('local-live', 'architect', '/repo'));
    liveSession.session = {
      id: 'saved-live',
      agentName: 'architect',
      sessionId: 'acp-remote-live',
      title: 'New live session',
      lastUpdated: 200,
      cwd: '/repo',
      supportsLoadSession: true,
    };
    liveSession.isConnected = true;

    store.sessions.set('local-stale', staleSession);
    store.sessions.set('local-live', liveSession);
    store.sessions = new Map(store.sessions);
    store.activeSessionId = null;

    const result = await store.focusOrResumeRemoteSession('acp-remote-live');

    expect(result).toEqual({ status: 'activated', localSessionId: 'local-live' });
    expect(store.activeSessionId).toBe('local-live');
    expect(mockBridge.loadSession).not.toHaveBeenCalled();
  });

  it('resumes a saved ACP session when no live local session exists', async () => {
    const store = useSessionStore();
    store.savedSessions = [
      {
        id: 'saved-remote',
        agentName: 'architect',
        sessionId: 'acp-remote-resume',
        title: 'Resumable session',
        lastUpdated: Date.now(),
        cwd: '/repo',
        supportsLoadSession: true,
      },
    ];

    const result = await store.focusOrResumeRemoteSession('acp-remote-resume');

    expect(result.status).toBe('resumed');
    expect(result.localSessionId).toEqual(expect.any(String));
    expect(store.activeSessionId).toBe(result.localSessionId);
    expect(store.currentSession?.sessionId).toBe('acp-remote-resume');
    expect(mockBridge.initialize).toHaveBeenCalledOnce();
    expect(mockBridge.loadSession).toHaveBeenCalledWith({
      sessionId: 'acp-remote-resume',
      cwd: '/repo',
      mcpServers: [],
    });
  });

  it('resumes through session/load when only a disconnected local match exists', async () => {
    const store = useSessionStore();
    const staleSession = reactive(createEmptySessionState('local-stale', 'architect', '/repo'));
    staleSession.session = {
      id: 'saved-remote',
      agentName: 'architect',
      sessionId: 'acp-remote-resume',
      title: 'Disconnected local session',
      lastUpdated: 100,
      cwd: '/repo',
      supportsLoadSession: true,
    };

    store.sessions.set('local-stale', staleSession);
    store.sessions = new Map(store.sessions);
    store.savedSessions = [
      {
        id: 'saved-remote',
        agentName: 'architect',
        sessionId: 'acp-remote-resume',
        title: 'Resumable session',
        lastUpdated: 200,
        cwd: '/repo',
        supportsLoadSession: true,
      },
    ];

    const result = await store.focusOrResumeRemoteSession('acp-remote-resume');

    expect(result.status).toBe('resumed');
    expect(result.localSessionId).not.toBe('local-stale');
    expect(store.activeSessionId).toBe(result.localSessionId);
    expect(mockBridge.loadSession).toHaveBeenCalledWith({
      sessionId: 'acp-remote-resume',
      cwd: '/repo',
      mcpServers: [],
    });
  });

  it('surfaces a handled error when the ACP session cannot be found', async () => {
    const store = useSessionStore();

    const result = await store.focusOrResumeRemoteSession('acp-remote-missing');

    expect(result).toEqual({ status: 'missing' });
    expect(store.error).toContain('ACP session acp-remote-missing is unavailable');
  });
});
