// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import type { VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { reactive } from 'vue';

import SessionList from '../SessionList.vue';
import { createEmptySessionState } from '../../lib/types';
import type { SavedSession, SessionState } from '../../lib/types';
import { useSessionStore } from '../../stores/session';
import { PINNED_SESSIONS_STORAGE_KEY } from '../../stores/pinned-sessions';

const readResource = vi.fn();
const subscribeResource = vi.fn();

vi.mock('../../composables/useMcpResources', () => ({
  useMcpResources: () => ({
    readResource,
    subscribeResource,
  }),
}));

beforeEach(() => {
  document.body.innerHTML = '';
  window.localStorage.clear();
  readResource.mockReset();
  subscribeResource.mockReset();
  readResource.mockResolvedValue({
    uri: 'agent://sessions',
    text: JSON.stringify({ sessions: [], generatedAt: new Date().toISOString() }),
  });
  subscribeResource.mockResolvedValue(vi.fn());
});

function createSavedSession(sessionId = 'acp-remote-target', overrides: Partial<SavedSession> = {}): SavedSession {
  return {
    id: 'saved-test',
    agentName: 'architect',
    sessionId,
    title: 'Saved session',
    lastUpdated: Date.now(),
    cwd: '/repo',
    supportsLoadSession: true,
    ...overrides,
  };
}

function createLocalSession(isConnected: boolean, remoteSessionId = 'acp-remote-target'): SessionState {
  const session = reactive(createEmptySessionState('local-existing', 'architect', '/repo'));
  session.session = createSavedSession(remoteSessionId);
  session.isConnected = isConnected;
  return session;
}

function mountSessionList(savedSession: SavedSession, localSessions = new Map<string, SessionState>()) {
  const pinia = createPinia();
  setActivePinia(pinia);

  const sessionStore = useSessionStore();
  sessionStore.savedSessions = [savedSession];
  sessionStore.sessions = localSessions;
  const setActiveSession = vi.spyOn(sessionStore, 'setActiveSession');

  const wrapper = mount(SessionList, {
    attachTo: document.body,
    global: {
      plugins: [pinia],
    },
  });

  return {
    setActiveSession,
    wrapper,
  };
}

function mountWithSavedSessions(savedSessions: SavedSession[]) {
  const pinia = createPinia();
  setActivePinia(pinia);

  const sessionStore = useSessionStore();
  sessionStore.savedSessions = savedSessions;
  sessionStore.sessions = new Map();

  const wrapper = mount(SessionList, {
    attachTo: document.body,
    global: { plugins: [pinia] },
  });

  return wrapper;
}

function sessionTitles(wrapper: VueWrapper): string[] {
  return wrapper.findAll('li[role="listitem"]').map((item) => item.find('span[title]').text());
}

function sessionRow(wrapper: VueWrapper, title: string) {
  const row = wrapper.findAll('li[role="listitem"]').find((item) => item.text().includes(title));
  if (!row) throw new Error(`Missing row for ${title}`);
  return row;
}

describe('SessionList resume click handling', () => {
  it('activates a matching connected local session without emitting resume', async () => {
    const savedSession = createSavedSession();
    const { setActiveSession, wrapper } = mountSessionList(
      savedSession,
      new Map([['local-connected', createLocalSession(true)]]),
    );

    await wrapper.get('li[role="listitem"]').trigger('click');

    expect(setActiveSession).toHaveBeenCalledWith('local-connected');
    expect(wrapper.emitted('resume')).toBeUndefined();
    wrapper.unmount();
  });

  it('emits resume when the matching local session is disconnected', async () => {
    const savedSession = createSavedSession();
    const { setActiveSession, wrapper } = mountSessionList(
      savedSession,
      new Map([['local-disconnected', createLocalSession(false)]]),
    );

    await wrapper.get('li[role="listitem"]').trigger('click');

    expect(setActiveSession).not.toHaveBeenCalled();
    expect(wrapper.emitted('resume')).toEqual([[savedSession]]);
    wrapper.unmount();
  });

  it('emits resume when no local session matches', async () => {
    const savedSession = createSavedSession();
    const { setActiveSession, wrapper } = mountSessionList(
      savedSession,
      new Map([['local-other', createLocalSession(true, 'acp-remote-other')]]),
    );

    await wrapper.get('li[role="listitem"]').trigger('click');

    expect(setActiveSession).not.toHaveBeenCalled();
    expect(wrapper.emitted('resume')).toEqual([[savedSession]]);
    wrapper.unmount();
  });
});

describe('SessionList pinned sessions', () => {
  it('renders a pin button for each session row', () => {
    const sessions = [
      createSavedSession('id-a', { id: 'a', title: 'Alpha' }),
      createSavedSession('id-b', { id: 'b', title: 'Beta' }),
    ];
    const wrapper = mountWithSavedSessions(sessions);

    expect(wrapper.findAll('button[aria-label="Pin session"]')).toHaveLength(2);
    wrapper.unmount();
  });

  it('pins a session, persists it, and moves it to the top', async () => {
    const alpha = createSavedSession('id-a', { id: 'alpha', title: 'Alpha', lastUpdated: 1 });
    const beta = createSavedSession('id-b', { id: 'beta', title: 'Beta', lastUpdated: 2 });
    const wrapper = mountWithSavedSessions([alpha, beta]);

    expect(sessionTitles(wrapper)).toEqual(['Beta', 'Alpha']);
    await sessionRow(wrapper, 'Alpha').get('button[aria-label="Pin session"]').trigger('click');

    expect(sessionTitles(wrapper)).toEqual(['Alpha', 'Beta']);
    expect(window.localStorage.getItem(PINNED_SESSIONS_STORAGE_KEY)).toBe('["alpha"]');
    wrapper.unmount();
  });

  it('hydrates pinned sessions from localStorage and sorts them first', () => {
    window.localStorage.setItem(PINNED_SESSIONS_STORAGE_KEY, '["alpha"]');
    const alpha = createSavedSession('id-a', { id: 'alpha', title: 'Alpha', lastUpdated: 1 });
    const beta = createSavedSession('id-b', { id: 'beta', title: 'Beta', lastUpdated: 2 });
    const wrapper = mountWithSavedSessions([alpha, beta]);

    expect(sessionTitles(wrapper)).toEqual(['Alpha', 'Beta']);
    expect(sessionRow(wrapper, 'Alpha').get('button[aria-label="Unpin session"]').attributes('aria-pressed')).toBe('true');
    wrapper.unmount();
  });

  it('unpins a pinned session and restores newest-first ordering', async () => {
    window.localStorage.setItem(PINNED_SESSIONS_STORAGE_KEY, '["alpha"]');
    const alpha = createSavedSession('id-a', { id: 'alpha', title: 'Alpha', lastUpdated: 1 });
    const beta = createSavedSession('id-b', { id: 'beta', title: 'Beta', lastUpdated: 2 });
    const wrapper = mountWithSavedSessions([alpha, beta]);

    await sessionRow(wrapper, 'Alpha').get('button[aria-label="Unpin session"]').trigger('click');

    expect(sessionTitles(wrapper)).toEqual(['Beta', 'Alpha']);
    expect(window.localStorage.getItem(PINNED_SESSIONS_STORAGE_KEY)).toBe('[]');
    wrapper.unmount();
  });
});

describe('SessionList search filter', () => {
  it('filters the session list by case-insensitive title substring', async () => {
    const sessionA = createSavedSession('id-a', { id: 'a', title: 'Alpha deployment' });
    const sessionB = createSavedSession('id-b', { id: 'b', title: 'Beta workflow' });
    const wrapper = mountWithSavedSessions([sessionA, sessionB]);

    const input = wrapper.find('input');
    await input.setValue('alpha');
    await wrapper.vm.$nextTick();

    const items = wrapper.findAll('li[role="listitem"]');
    expect(items).toHaveLength(1);
    expect(items[0]!.text()).toContain('Alpha deployment');
    wrapper.unmount();
  });

  it('restores the full list when the input is cleared', async () => {
    const sessionA = createSavedSession('id-a', { id: 'a', title: 'Alpha deployment' });
    const sessionB = createSavedSession('id-b', { id: 'b', title: 'Beta workflow' });
    const wrapper = mountWithSavedSessions([sessionA, sessionB]);

    const input = wrapper.find('input');
    await input.setValue('alpha');
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll('li[role="listitem"]')).toHaveLength(1);

    await input.setValue('');
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll('li[role="listitem"]')).toHaveLength(2);
    wrapper.unmount();
  });

  it('treats whitespace-only input as empty and shows all sessions', async () => {
    const sessionA = createSavedSession('id-a', { id: 'a', title: 'Alpha deployment' });
    const sessionB = createSavedSession('id-b', { id: 'b', title: 'Beta workflow' });
    const wrapper = mountWithSavedSessions([sessionA, sessionB]);

    const input = wrapper.find('input');
    await input.setValue('   ');
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll('li[role="listitem"]')).toHaveLength(2);
    wrapper.unmount();
  });
});
