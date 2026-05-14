// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import SessionList from '../SessionList.vue';
import type { SamplingHistoryEntry, SavedSession } from '../../lib/types';
import { useSessionStore } from '../../stores/session';

const readResource = vi.fn();
const subscribeResource = vi.fn();

class MockEventSource {
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  close = vi.fn();
}

vi.mock('../../composables/useMcpResources', () => ({
  useMcpResources: () => ({
    readResource,
    subscribeResource,
  }),
}));

function savedSession(): SavedSession {
  return {
    id: 'agent-row',
    agentName: 'architect',
    sessionId: 'agent-session-1',
    title: 'Agent planning session',
    lastUpdated: Date.parse('2026-05-13T12:01:00.000Z'),
    cwd: '/repo',
    supportsLoadSession: true,
  };
}

function samplingEntry(): SamplingHistoryEntry {
  return {
    samplingRequestId: '00000000-0000-4000-8000-000000000001',
    appSessionId: '10000000-0000-4000-8000-000000000001',
    acpSessionId: 'acp-1',
    serverName: 'github',
    model: 'gpt-5.4',
    runtimeKind: 'copilot',
    roleName: 'copilot',
    status: 'streaming',
    decision: 'allow_once',
    createdAt: '2026-05-13T12:00:00.000Z',
    tokenUsage: null,
  };
}

async function mountMixedList() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const agent = savedSession();
  const sampling = samplingEntry();

  readResource.mockImplementation(async (uri: string) => {
    if (uri === 'sampling://sessions') {
      return { uri, text: JSON.stringify({ sessions: [sampling] }) };
    }
    return { uri, text: JSON.stringify({ sessions: [] }) };
  });
  subscribeResource.mockResolvedValue(vi.fn());

  const sessionStore = useSessionStore();
  sessionStore.savedSessions = [agent];
  sessionStore.sessions = new Map();

  const wrapper = mount(SessionList, {
    attachTo: document.body,
    global: { plugins: [pinia] },
  });
  await flushPromises();
  return wrapper;
}

function rows(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll('li[role="listitem"]');
}

describe('SessionList sampling rows', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    window.localStorage.clear();
    readResource.mockReset();
    subscribeResource.mockReset();
    vi.stubGlobal('EventSource', MockEventSource);
  });

  it('interleaves agent and sampling rows', async () => {
    const wrapper = await mountMixedList();

    expect(wrapper.text()).toContain('Agent planning session');
    expect(wrapper.text()).toContain('github');
    expect(wrapper.text()).toContain('agent');
    expect(wrapper.text()).toContain('sampling');
    wrapper.unmount();
  });

  it('filter chip All shows everything', async () => {
    const wrapper = await mountMixedList();

    await wrapper.get('button[aria-pressed="true"]').trigger('click');

    expect(rows(wrapper)).toHaveLength(2);
    expect(wrapper.text()).toContain('Agent planning session');
    expect(wrapper.text()).toContain('github');
    wrapper.unmount();
  });

  it('filter chip Agent hides sampling rows', async () => {
    const wrapper = await mountMixedList();

    await wrapper.findAll('button').find((button) => button.text() === 'agent')!.trigger('click');

    expect(rows(wrapper)).toHaveLength(1);
    expect(wrapper.text()).toContain('Agent planning session');
    expect(wrapper.text()).not.toContain('github');
    wrapper.unmount();
  });

  it('filter chip Sampling hides agent rows', async () => {
    const wrapper = await mountMixedList();

    await wrapper.findAll('button').find((button) => button.text() === 'sampling')!.trigger('click');

    expect(rows(wrapper)).toHaveLength(1);
    expect(wrapper.text()).toContain('github');
    expect(wrapper.text()).not.toContain('Agent planning session');
    wrapper.unmount();
  });
});
