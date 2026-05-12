// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import GateTaskDetailHistory from '../GateTaskDetailHistory.vue';
import { getMarkdownIt } from '../../composables/useMarkdownIt';
import type { GateTaskHistoryEntry } from '../../lib/gate';

const pendingPass: GateTaskHistoryEntry = {
  stage: 'pending',
  event: 'PASS',
  agent: 'ralph-pm',
  model: null,
  notes: 'PM advancing pending to swe.',
  timestamp: '2026-05-01T08:00:00.000Z',
};

const swePass: GateTaskHistoryEntry = {
  stage: 'swe',
  event: 'PASS',
  agent: 'copilot-swe',
  model: 'claude-opus-4.7',
  notes: 'SWE stage complete.',
  timestamp: '2026-05-01T09:00:00.000Z',
};

const sweAppend: GateTaskHistoryEntry = {
  stage: 'swe',
  event: 'APPEND',
  agent: 'copilot-swe',
  model: null,
  notes: 'Appended extra context.',
  timestamp: '2026-05-01T09:30:00.000Z',
};

const history: GateTaskHistoryEntry[] = [pendingPass, swePass, sweAppend];

async function settle(): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await flushPromises();
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 10));
  }
}

describe('GateTaskDetailHistory', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    setActivePinia(createPinia());
    document.body.innerHTML = '';
  });

  it('renders stage groups and per-record fields for all entries', async () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history, historyLoading: false, historyError: '' },
      attachTo: document.body,
    });
    await settle();

    expect(wrapper.find('[data-testid="gate-history-stage-pending"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="gate-history-stage-swe"]').exists()).toBe(true);

    const text = wrapper.text();
    expect(text).toContain('ralph-pm');
    expect(text).toContain('copilot-swe');
    expect(text).toContain('claude-opus-4.7');
    expect(text).toContain('PASS');
    expect(text).toContain('APPEND');

    wrapper.unmount();
  });

  it('sort toggle reverses record order within stages', async () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history, historyLoading: false, historyError: '' },
      attachTo: document.body,
    });
    await settle();

    const sortBtn = wrapper.find('[data-testid="gate-history-toggle-sort"]');
    expect(sortBtn.exists()).toBe(true);
    expect(sortBtn.text()).toBe('Newest first');

    await sortBtn.trigger('click');
    await settle();

    expect(sortBtn.text()).toBe('Oldest first');

    await sortBtn.trigger('click');
    await settle();

    expect(sortBtn.text()).toBe('Newest first');

    wrapper.unmount();
  });

  it('filter toggle hides APPEND records', async () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history, historyLoading: false, historyError: '' },
      attachTo: document.body,
    });
    await settle();

    expect(wrapper.text()).toContain('APPEND');

    const filterBtn = wrapper.find('[data-testid="gate-history-toggle-append"]');
    expect(filterBtn.exists()).toBe(true);
    expect(filterBtn.text()).toBe('Hide appends');

    await filterBtn.trigger('click');
    await settle();

    expect(wrapper.text()).not.toContain('APPEND');
    expect(filterBtn.text()).toBe('Show all');

    await filterBtn.trigger('click');
    await settle();

    expect(wrapper.text()).toContain('APPEND');

    wrapper.unmount();
  });

  it('renders notes via MarkdownView (finds rendered markdown content)', async () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history, historyLoading: false, historyError: '' },
      attachTo: document.body,
    });
    await settle();

    expect(wrapper.text()).toContain('PM advancing pending to swe.');
    expect(wrapper.text()).toContain('SWE stage complete.');

    wrapper.unmount();
  });

  it('shows loading state', () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history: [], historyLoading: true, historyError: '' },
    });
    expect(wrapper.text()).toContain('Loading task history');
    wrapper.unmount();
  });

  it('shows error state', () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history: [], historyLoading: false, historyError: 'Connection refused' },
    });
    expect(wrapper.text()).toContain('Connection refused');
    wrapper.unmount();
  });

  it('shows empty state when no history', () => {
    const wrapper = mount(GateTaskDetailHistory, {
      props: { history: [], historyLoading: false, historyError: '' },
    });
    expect(wrapper.text()).toContain('No recorded gate history yet.');
    wrapper.unmount();
  });
});
