// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import GateViewContent from '../GateViewContent.vue';
import type { GateTask } from '../../lib/gate';

const task: GateTask = {
  taskId: 'T-250.2',
  canonicalTaskId: 'T-250.2',
  title: 'Conditional detail panel',
  pipeline: 'sdd',
  state: 'swe',
  stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
  attempts: 0,
  location: 'todo',
  path: '/repo/todo/T-250.2.md',
  issue: '',
};

const stubs = {
  ResizablePanelGroup: { template: '<div><slot /></div>' },
  ResizablePanel: { template: '<div><slot /></div>' },
  ResizableHandle: { template: '<div />' },
  GateTaskTable: { template: '<div />' },
  GateTaskDetail: { template: '<div />' },
};

const baseProps = {
  tasks: [],
  selectedTaskKey: '',
  sortColumn: 'id' as const,
  sortDirection: 'asc' as const,
  activeTask: null,
  body: '',
  history: [],
  bodyLoading: false,
  historyLoading: false,
  bodyError: '',
  historyError: '',
  root: '/repo',
};

describe('GateViewContent', () => {
  it('does not render the detail panel when selectedTaskKey is empty', () => {
    const wrapper = mount(GateViewContent, {
      props: { ...baseProps },
      global: { stubs },
    });

    expect(wrapper.find('[data-testid="gate-detail-panel"]').exists()).toBe(false);
  });

  it('renders the detail panel when selectedTaskKey is non-empty', () => {
    const wrapper = mount(GateViewContent, {
      props: { ...baseProps, selectedTaskKey: 'T-250.2', activeTask: task },
      global: { stubs },
    });

    expect(wrapper.find('[data-testid="gate-detail-panel"]').exists()).toBe(true);
  });
});
