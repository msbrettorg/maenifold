// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import WorkflowRunButton from '../WorkflowRunButton.vue';
import { useWorkflowStore } from '../../stores/workflow';

const gatewayExecute = vi.fn();

vi.mock('../../composables/useSharedMcpClient', () => ({
  getSharedMcpClient: () => ({
    gatewayExecute,
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
    gatewayExecute,
    connected: { value: true },
    connect: vi.fn(),
    error: { value: null },
  }),
}));

const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div><slot /></div>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<div><slot /></div>' },
  DialogHeader: { template: '<div><slot /></div>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  Select: { template: '<div><slot /></div>', props: ['modelValue'] },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: { template: '<button type="button"><slot /></button>', props: ['value'] },
  SelectTrigger: { template: '<button type="button"><slot /></button>' },
  SelectValue: { template: '<span><slot /></span>', props: ['placeholder'] },
};

function workflow(inputs?: unknown) {
  return {
    id: 'demo-workflow',
    name: 'Demo workflow',
    emoji: '🧪',
    description: '',
    triggers: [],
    sequentialThinkingEnabled: false,
    multiAgent: false,
    steps: [{
      id: 'step-1',
      name: 'Do work',
      description: 'Work',
      requiresSequentialThinking: false,
    }],
    ...(inputs === undefined ? {} : { inputs }),
  };
}

describe('WorkflowRunButton', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    gatewayExecute.mockReset();
    gatewayExecute.mockResolvedValue({
      sessionId: 'wf-session-1',
      message: 'Created session: wf-session-1',
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('dispatches a no-input workflow immediately', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    const wrapper = mount(WorkflowRunButton, {
      props: { workflow: workflow() },
      global: { plugins: [pinia], stubs: dialogStubs },
    });

    await wrapper.get('[data-testid="workflow-run-button"]').trigger('click');

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.workflow', 'Workflow', { workflowId: 'demo-workflow' });
    expect(store.selectedSessionId).toBe('wf-session-1');
  });

  it('submits validated parameter dialog inputs', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const wrapper = mount(WorkflowRunButton, {
      props: {
        workflow: workflow({
          topic: { label: 'Topic', type: 'text', required: true },
          notes: { label: 'Notes', type: 'textarea' },
        }),
      },
      global: { plugins: [pinia], stubs: dialogStubs },
    });

    await wrapper.get('[data-testid="workflow-run-button"]').trigger('click');
    await wrapper.get('[data-testid="workflow-parameter-submit"]').trigger('click');
    expect(wrapper.text()).toContain('Topic is required.');

    await wrapper.get('[name="topic"]').setValue('cost optimization');
    await wrapper.get('[name="notes"]').setValue('include reservations');
    await nextTick();
    await wrapper.get('[data-testid="workflow-parameter-submit"]').trigger('click');

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.workflow', 'Workflow', {
      workflowId: 'demo-workflow',
      inputs: {
        topic: 'cost optimization',
        notes: 'include reservations',
      },
    });
  });
});
