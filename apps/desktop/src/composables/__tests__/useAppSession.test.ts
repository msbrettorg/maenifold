// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { reactive } from 'vue';

import { createEmptySessionState } from '../../lib/types';
import type { AgentsConfig, SavedSession, SessionState } from '../../lib/types';
import { useSessionStore } from '../../stores/session';
import type { ConfigStoreLike } from '../useAppSession';
import { useAppSession } from '../useAppSession';

vi.mock('../../stores/agent-capabilities', () => ({
  useAgentCapabilitiesStore: () => ({
    syncFromConfigAgents: vi.fn(),
    probeConfiguredAgents: vi.fn(),
  }),
}));

vi.mock('../../stores/agent-session-overrides', () => ({
  useAgentSessionOverridesStore: () => ({
    getSelectedOverrides: vi.fn(() => ({})),
    clearSelectedOverrides: vi.fn(),
  }),
}));

function createSavedSession(sessionId = 'acp-remote-target'): SavedSession {
  return {
    id: 'saved-test',
    agentName: 'architect',
    sessionId,
    title: 'Saved session',
    lastUpdated: Date.now(),
    cwd: '/repo',
    supportsLoadSession: true,
  };
}

function createLocalSession(isConnected: boolean, remoteSessionId = 'acp-remote-target'): SessionState {
  const session = reactive(createEmptySessionState('local-existing', 'architect', '/repo'));
  session.session = createSavedSession(remoteSessionId);
  session.isConnected = isConnected;
  return session;
}

function createConfigStore(): ConfigStoreLike {
  return {
    error: null,
    hasAgents: true,
    config: { agents: { architect: { command: 'architect' } } } as AgentsConfig,
    agentNames: ['architect'],
    loadConfig: vi.fn().mockResolvedValue(undefined),
    setupHotReload: vi.fn().mockResolvedValue(undefined),
    clearError: vi.fn(),
  };
}

function createTestSessionStore(sessions = new Map<string, SessionState>()) {
  const sessionStore = useSessionStore();
  sessionStore.sessions = sessions;

  return {
    sessionStore,
    resumeSession: vi.spyOn(sessionStore, 'resumeSession').mockResolvedValue(undefined),
    setActiveSession: vi.spyOn(sessionStore, 'setActiveSession').mockImplementation(() => undefined),
  };
}

describe('useAppSession.handleResumeSession', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    window.localStorage.clear();
    vi.clearAllMocks();
  });

  it('activates a matching connected local session without resuming', async () => {
    const savedSession = createSavedSession();
    const { sessionStore, resumeSession, setActiveSession } = createTestSessionStore(new Map([
      ['local-connected', createLocalSession(true)],
    ]));
    const appSession = useAppSession({
      configStore: createConfigStore(),
      sessionStore,
    });

    await appSession.handleResumeSession(savedSession);

    expect(setActiveSession).toHaveBeenCalledWith('local-connected');
    expect(resumeSession).not.toHaveBeenCalled();
  });

  it('resumes when the matching local session is disconnected', async () => {
    const savedSession = createSavedSession();
    const { sessionStore, resumeSession, setActiveSession } = createTestSessionStore(new Map([
      ['local-disconnected', createLocalSession(false)],
    ]));
    const appSession = useAppSession({
      configStore: createConfigStore(),
      sessionStore,
    });

    await appSession.handleResumeSession(savedSession);

    expect(setActiveSession).not.toHaveBeenCalled();
    expect(resumeSession).toHaveBeenCalledWith(savedSession);
  });

  it('resumes when no local session matches', async () => {
    const savedSession = createSavedSession();
    const { sessionStore, resumeSession, setActiveSession } = createTestSessionStore(new Map([
      ['local-other', createLocalSession(true, 'acp-remote-other')],
    ]));
    const appSession = useAppSession({
      configStore: createConfigStore(),
      sessionStore,
    });

    await appSession.handleResumeSession(savedSession);

    expect(setActiveSession).not.toHaveBeenCalled();
    expect(resumeSession).toHaveBeenCalledWith(savedSession);
  });
});
