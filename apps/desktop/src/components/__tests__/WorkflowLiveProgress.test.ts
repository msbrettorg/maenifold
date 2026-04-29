// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import WorkflowLiveProgress from '../WorkflowLiveProgress.vue';
import { useWorkflowStore } from '../../stores/workflow';

const callTool = vi.fn();

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    callTool,
    connected: { value: true },
    connect: vi.fn(),
    error: { value: null },
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
    unsubscribeResource: vi.fn(),
    supportsResourceSubscribe: vi.fn(() => true),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
  }),
  usePipelineMcp: () => ({
    callTool,
    connected: { value: true },
    connect: vi.fn(),
    error: { value: null },
  }),
}));

function session(status: string, currentStep: number) {
  return {
    id: 'wf-session-1',
    uri: 'memory://thinking/workflow/wf-session-1',
    status,
    workflow_id: 'demo-workflow',
    current_step: currentStep,
    total_steps: 2,
    started_at: '2026-04-29T12:00:00Z',
    updated_at: '2026-04-29T12:00:03Z',
    history: [
      { heading: 'Step 1/2', body: 'First prompt' },
      { heading: 'Step 1/2 Response', body: 'First output' },
      { heading: 'Step 2/2', body: 'Second prompt' },
    ],
  };
}

describe('WorkflowLiveProgress', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    callTool.mockReset();
    callTool.mockResolvedValue('Cancelled');
  });

  it('renders running and completed status transitions', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSession = session('active', 2);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain('running');
    expect(wrapper.text()).toContain('completed');
    expect(wrapper.text()).not.toContain('Run completed');

    store.selectedSession = session('completed', 2);
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('Run completed');
    expect(wrapper.text()).toContain('Total duration');
  });

  it('calls Workflow cancellation from the stop button', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSession = session('active', 1);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    await wrapper.get('[data-testid="workflow-stop-button"]').trigger('click');

    expect(callTool).toHaveBeenCalledWith('Workflow', {
      sessionId: 'wf-session-1',
      response: 'Cancelled from desktop workflow runner. [[workflow-execution]]',
      status: 'cancelled',
    });
  });
});
