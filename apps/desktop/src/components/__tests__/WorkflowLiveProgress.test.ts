// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { enableAutoUnmount, flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import WorkflowLiveProgress from '../WorkflowLiveProgress.vue';
import { useWorkflowStore } from '../../stores/workflow';

enableAutoUnmount(afterEach);

const mcpMock = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  const notificationHandlers: Array<(notification: { method: string; params?: unknown }) => void> = [];
  return {
    callTool: vi.fn(),
    connect: vi.fn(),
    connected: vue.ref(true),
    error: vue.ref<string | null>(null),
    notificationHandlers,
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
    unsubscribeResource: vi.fn(),
    emitResourceUpdate(uri: string) {
      notificationHandlers.forEach((handler) => {
        handler({ method: 'notifications/resources/updated', params: { uri } });
      });
    },
  };
});

function resourcePayload(uri: string) {
  return {
    uri,
    text: uri === 'workflow://sessions'
      ? JSON.stringify({ sessions: [session('active', 2)], generated_at: '2026-04-29T12:00:04Z' })
      : JSON.stringify(session('active', 2)),
  };
}

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    callTool: mcpMock.callTool,
    connected: mcpMock.connected,
    connect: mcpMock.connect,
    error: mcpMock.error,
    readResource: mcpMock.readResource,
    subscribeResource: mcpMock.subscribeResource,
    unsubscribeResource: mcpMock.unsubscribeResource,
    supportsResourceSubscribe: vi.fn(() => true),
    onNotification: vi.fn((handler) => {
      mcpMock.notificationHandlers.push(handler);
      return () => {
        const index = mcpMock.notificationHandlers.indexOf(handler);
        if (index >= 0) mcpMock.notificationHandlers.splice(index, 1);
      };
    }),
    onDisconnect: vi.fn(() => vi.fn()),
  }),
  usePipelineMcp: () => ({
    callTool: mcpMock.callTool,
    connected: mcpMock.connected,
    connect: mcpMock.connect,
    error: mcpMock.error,
  }),
}));

function session(status: string, currentStep: number, stepTimings = {}, currentWorkflowIndex = 0) {
  return {
    id: 'wf-session-1',
    uri: 'memory://thinking/workflow/wf-session-1',
    status,
    workflow_id: 'demo-workflow',
    current_workflow_index: currentWorkflowIndex,
    current_step: currentStep,
    total_steps: 2,
    started_at: '2026-04-29T12:00:00Z',
    updated_at: '2026-04-29T12:00:03Z',
    history: [
      { heading: 'Step 1/2', body: 'First prompt' },
      { heading: 'Step 1/2 Response', body: 'First output' },
      { heading: 'Step 2/2', body: 'Second prompt' },
    ],
    step_timings: stepTimings,
  };
}

describe('WorkflowLiveProgress', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mcpMock.callTool.mockReset();
    mcpMock.callTool.mockResolvedValue('Cancelled');
    mcpMock.connect.mockReset();
    mcpMock.connect.mockImplementation(() => {
      mcpMock.connected.value = true;
      return Promise.resolve();
    });
    mcpMock.connected.value = true;
    mcpMock.error.value = null;
    mcpMock.readResource.mockReset();
    mcpMock.readResource.mockImplementation((uri: string) => Promise.resolve(resourcePayload(uri)));
    mcpMock.subscribeResource.mockReset();
    mcpMock.subscribeResource.mockResolvedValue(vi.fn());
    mcpMock.unsubscribeResource.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders running and completed status transitions', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
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
    expect(wrapper.text()).not.toContain('Updated');
    expect(wrapper.text()).toContain('Total duration');
  });

  it('calls Workflow cancellation from the stop button', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 1);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    await wrapper.get('[data-testid="workflow-stop-button"]').trigger('click');

    expect(mcpMock.callTool).toHaveBeenCalledWith('Workflow', {
      sessionId: 'wf-session-1',
      response: 'Cancelled from desktop workflow runner. [[workflow-execution]]',
      status: 'cancelled',
    });
  });

  it('renders disconnect banner when MCP disconnects', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 2);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    await flushPromises();
    mcpMock.connected.value = false;
    await flushPromises();

    expect(wrapper.text()).toContain('Lost connection to daemon. Run state may be stale.');
    expect(wrapper.text()).toContain('Retry');
  });

  it('Retry button calls reconnect', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 2);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    await flushPromises();
    mcpMock.connected.value = false;
    await flushPromises();
    await wrapper.get('button').trigger('click');

    expect(mcpMock.connect).toHaveBeenCalledTimes(1);
  });

  it('marks run as stale after 60s without resource update', async () => {
    vi.useFakeTimers();
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 2);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    await vi.advanceTimersByTimeAsync(60_001);

    expect(wrapper.text()).toContain('Stale');
  });

  it('stale timer resets when resource update arrives', async () => {
    vi.useFakeTimers();
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 2);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    await vi.advanceTimersByTimeAsync(30_000);
    mcpMock.emitResourceUpdate('workflow://session/wf-session-1');
    await flushPromises();
    await vi.advanceTimersByTimeAsync(30_001);

    expect(wrapper.text()).not.toContain('Stale');
  });

  it('re-subscribes on reconnect', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 2);
    const resubscribeSpy = vi.spyOn(store, 'resubscribe');

    mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    mcpMock.connected.value = false;
    await flushPromises();
    mcpMock.connected.value = true;
    await flushPromises();

    expect(resubscribeSpy).toHaveBeenCalledTimes(1);
  });

  it('reconnect resubscribe is idempotent under rapid flapping', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSessionId = 'wf-session-1';
    store.selectedSession = session('active', 2);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });
    await flushPromises();

    let releaseRead: () => void = () => {
      throw new Error('resubscribe did not start reading resources');
    };
    let holdFirstRead = true;
    mcpMock.subscribeResource.mockClear();
    mcpMock.readResource.mockImplementation((uri: string) => {
      if (!holdFirstRead) {
        return Promise.resolve(resourcePayload(uri));
      }
      holdFirstRead = false;
      return new Promise((resolve) => {
        releaseRead = () => resolve(resourcePayload(uri));
      });
    });

    mcpMock.connected.value = false;
    await wrapper.vm.$nextTick();
    mcpMock.connected.value = true;
    await wrapper.vm.$nextTick();
    mcpMock.connected.value = false;
    await wrapper.vm.$nextTick();
    mcpMock.connected.value = true;
    await wrapper.vm.$nextTick();

    expect(mcpMock.subscribeResource).not.toHaveBeenCalled();
    expect(releaseRead).toEqual(expect.any(Function));
    releaseRead();
    await flushPromises();
    await flushPromises();
    await flushPromises();

    expect(mcpMock.subscribeResource).toHaveBeenCalledTimes(2);
    expect(mcpMock.subscribeResource).toHaveBeenNthCalledWith(1, 'workflow://sessions');
    expect(mcpMock.subscribeResource).toHaveBeenNthCalledWith(2, 'workflow://session/wf-session-1');
  });

  it('renders per-step duration when both timings present', () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSession = session('active', 2, {
      '1': {
        started_at: '2026-04-29T12:00:00Z',
        ended_at: '2026-04-29T12:00:01.200Z',
      },
    });

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain('Duration: 1.2s');
  });

  it('renders per-step duration for second workflow in queue', () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSession = session('active', 1, {
      '2:1': {
        started_at: '2026-04-29T12:00:00Z',
        ended_at: '2026-04-29T12:00:05Z',
      },
    }, 1);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain('Duration: 5s');
  });

  it('hides duration label when step has no timing', () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const store = useWorkflowStore();
    store.selectedSession = session('active', 1);

    const wrapper = mount(WorkflowLiveProgress, {
      props: { workflowName: 'Demo workflow' },
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).not.toContain('Duration:');
  });
});
