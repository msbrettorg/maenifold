// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';

import SessionRowActions from '../SessionRowActions.vue';
import type { SamplingHistoryEntry } from '../../lib/types';

function samplingSession(status: string): SamplingHistoryEntry {
  return {
    samplingRequestId: '00000000-0000-4000-8000-000000000001',
    appSessionId: '10000000-0000-4000-8000-000000000001',
    acpSessionId: 'acp-1',
    serverName: 'github',
    model: 'gpt-5.4',
    runtimeKind: 'copilot',
    roleName: 'copilot',
    status,
    decision: 'allow_once',
    createdAt: '2026-05-13T12:00:00.000Z',
    tokenUsage: null,
  };
}

function mountSamplingActions(status: string) {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(SessionRowActions, {
    attachTo: document.body,
    props: {
      session: samplingSession(status),
      source: 'sampling',
      canStop: status === 'dispatching' || status === 'streaming',
    },
    global: { plugins: [pinia] },
  });
}

describe('SessionRowActions sampling stop', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    window.localStorage.clear();
  });

  it('Stop button visible only for Dispatching/Streaming sampling rows', () => {
    for (const status of ['pending_approval', 'completed', 'cancelled', 'denied', 'failed']) {
      const wrapper = mountSamplingActions(status);
      expect(wrapper.find('button[aria-label="Stop sampling session"]').exists()).toBe(false);
      wrapper.unmount();
    }

    for (const status of ['dispatching', 'streaming']) {
      const wrapper = mountSamplingActions(status);
      expect(wrapper.find('button[aria-label="Stop sampling session"]').exists()).toBe(true);
      wrapper.unmount();
    }
  });

  it('Stop click emits stop event with correct payload', async () => {
    const wrapper = mountSamplingActions('streaming');

    await wrapper.get('button[aria-label="Stop sampling session"]').trigger('click');

    expect(wrapper.emitted('stop')?.[0]?.[0]).toBe('00000000-0000-4000-8000-000000000001');
    wrapper.unmount();
  });
});
