// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import SessionList from '../SessionList.vue';
import type { SavedSession } from '../../lib/types';
import { useSessionStore } from '../../stores/session';

const agentStore = {
  activeSessionMap: new Map(),
  start: vi.fn(),
  stop: vi.fn(),
};

vi.mock('../../stores/agent', () => ({
  useAgentStore: () => agentStore,
}));

function savedSession(overrides: Partial<SavedSession> = {}): SavedSession {
  return {
    id: 'agent-row',
    agentName: 'architect',
    sessionId: 'agent-session-1',
    title: 'Agent planning session',
    lastUpdated: Date.parse('2026-05-13T12:01:00.000Z'),
    cwd: '/repo',
    supportsLoadSession: true,
    ...overrides,
  };
}

async function mountList(sessions: SavedSession[]) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const sessionStore = useSessionStore();
  sessionStore.savedSessions = sessions;
  sessionStore.sessions = new Map();

  return mount(SessionList, {
    attachTo: document.body,
    global: { plugins: [pinia] },
  });
}

function rows(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll('li[role="listitem"]');
}

describe('SessionList', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    window.localStorage.clear();
    agentStore.activeSessionMap = new Map();
    agentStore.start.mockReset();
    agentStore.stop.mockReset();
  });

  it('renders saved agent sessions', async () => {
    const wrapper = await mountList([savedSession()]);

    expect(wrapper.text()).toContain('Agent planning session');
    expect(wrapper.text()).toContain('architect');
    expect(rows(wrapper)).toHaveLength(1);
    wrapper.unmount();
  });

  it('does not render sampling filters or sampling rows in the Agents sidebar', async () => {
    const wrapper = await mountList([savedSession()]);

    expect(wrapper.find('[aria-label="Session source filter"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('sampling');
    expect(wrapper.text()).not.toContain('Request');
    expect(wrapper.text()).not.toContain('Response');
    wrapper.unmount();
  });

  it('filters agent sessions by title and agent name', async () => {
    const wrapper = await mountList([
      savedSession(),
      savedSession({
        id: 'agent-row-2',
        agentName: 'researcher',
        sessionId: 'agent-session-2',
        title: 'Evidence review',
      }),
    ]);

    await wrapper.get('input[aria-label="Filter sessions"]').setValue('review');

    expect(rows(wrapper)).toHaveLength(1);
    expect(wrapper.text()).toContain('Evidence review');
    expect(wrapper.text()).not.toContain('Agent planning session');
    wrapper.unmount();
  });
});
