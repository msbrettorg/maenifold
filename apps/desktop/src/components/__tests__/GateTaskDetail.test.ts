// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import GateTaskDetail from '../GateTaskDetail.vue';
import { getMarkdownIt } from '../../composables/useMarkdownIt';
import type { GateTask } from '../../lib/gate';
import { useAppShellStore } from '../../stores/app-shell';
import { useKnowledgeStore } from '../../stores/knowledge';

const task: GateTask = {
  taskId: 'T-243.71',
  canonicalTaskId: 'T-243.71',
  title: 'Gate WikiLinks navigate to Knowledge search',
  pipeline: 'sdd',
  state: 'done',
  stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
  attempts: 0,
  location: 'done',
  path: '/repo/done/T-243.71-gate-wikilinks.md',
  issue: '',
};

async function settle(): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await flushPromises();
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 10));
  }
}

describe('GateTaskDetail', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    setActivePinia(createPinia());
    document.body.innerHTML = '';
  });

  it('renders task body WikiLinks as Knowledge search affordances', async () => {
    const wrapper = mount(GateTaskDetail, {
      props: {
        task,
        body: 'Review [[Shared Concept]] before passing.',
        history: [],
        bodyLoading: false,
        historyLoading: false,
        bodyError: '',
        historyError: '',
        root: '/repo',
      },
      attachTo: document.body,
      global: {
        stubs: {
          GateTaskDetailHistory: { template: '<div />' },
          GateTaskDetailMeta: { template: '<div />' },
          GateTaskDetailStages: { template: '<div />' },
        },
      },
    });
    await settle();

    const link = wrapper.find('[data-wikilink="Shared Concept"]');
    expect(link.exists()).toBe(true);
    expect(link.element.tagName).toBe('A');

    await link.trigger('click');

    const appShell = useAppShellStore();
    const knowledge = useKnowledgeStore();
    expect(appShell.activeTab).toBe('knowledge');
    expect(knowledge.searchQuery).toBe('Shared Concept');
    expect(knowledge.activeTabUri).toBeNull();
    wrapper.unmount();
  });

  it('opens raw markdown guidance from the title edit affordance', async () => {
    const wrapper = mount(GateTaskDetail, {
      props: {
        task,
        body: 'Review the task title.',
        history: [],
        bodyLoading: false,
        historyLoading: false,
        bodyError: '',
        historyError: '',
        root: '/repo',
      },
      attachTo: document.body,
      global: {
        stubs: {
          GateTaskDetailHistory: { template: '<div />' },
          GateTaskDetailMeta: { template: '<div />' },
          GateTaskDetailStages: { template: '<div />' },
        },
      },
    });
    await settle();

    const affordance = wrapper.find('[data-testid="gate-task-title-edit"]');
    expect(affordance.exists()).toBe(true);
    expect(affordance.find('svg').exists()).toBe(true);

    await affordance.trigger('click');
    await settle();

    expect(document.body.textContent).toContain('Title editing requires backend gate_UpdateTitle (filed T-249.169).');
    expect(document.body.textContent).toContain('For now, edit the markdown file directly.');
    expect(document.body.textContent).toContain('/repo/done/T-243.71-gate-wikilinks.md');
    wrapper.unmount();
  });

  it('surfaces frontmatter blocked reason and superseding task IDs', async () => {
    const blockedTask: GateTask = {
      ...task,
      taskId: 'T-309.3',
      canonicalTaskId: 'T-309.3',
      title: 'Blocked gate task',
      state: 'blocked',
      location: 'todo',
      path: '/repo/todo/T-309.3-blocked.md',
      blocked: true,
      blocked_reason: 'Superseded by narrower implementation task.',
      superseded_by: ['T-322.14'],
    };
    const wrapper = mount(GateTaskDetail, {
      props: {
        task: blockedTask,
        body: 'Review blocked task metadata.',
        history: [],
        bodyLoading: false,
        historyLoading: false,
        bodyError: '',
        historyError: '',
        root: '/repo',
      },
      attachTo: document.body,
      global: {
        stubs: {
          GateTaskDetailHistory: { template: '<div />' },
          GateTaskDetailStages: { template: '<div />' },
          GateStageActions: { template: '<div />' },
        },
      },
    });
    await settle();

    expect(wrapper.text()).toContain('Blocked reason');
    expect(wrapper.text()).toContain('Superseded by narrower implementation task.');
    expect(wrapper.text()).toContain('Superseded by');
    expect(wrapper.text()).toContain('T-322.14');
    wrapper.unmount();
  });
});
