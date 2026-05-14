// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WorkflowView from '../WorkflowView.vue';
import { useWorkflowStore } from '../../stores/workflow';

vi.mock('../../composables/useMcpResources', () => ({
  useMcpResources: () => ({
    readResource: vi.fn(),
    subscribeResource: vi.fn(async () => vi.fn()),
  }),
}));

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    callTool: vi.fn(),
  }),
}));

function session(id: string, workflowId: string, status: string) {
  return {
    id,
    uri: `workflow://session/${id}`,
    status,
    workflow_id: workflowId,
    current_step: 1,
    total_steps: 2,
    started_at: '2026-05-14T12:00:00Z',
    updated_at: '2026-05-14T12:00:01Z',
  };
}

function mountWorkflowView(filterWorkflowId: string | null) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = useWorkflowStore();
  store.activeSessions = [
    session('session-a1', 'workflow-A', 'active'),
    session('session-b1', 'workflow-B', 'completed'),
    session('session-a2', 'workflow-A', 'cancelled'),
  ];
  vi.spyOn(store, 'mount').mockResolvedValue();
  vi.spyOn(store, 'unmount').mockImplementation(() => undefined);

  const wrapper = mount(WorkflowView, {
    props: { filterWorkflowId },
    global: { plugins: [pinia] },
  });
  return { store, wrapper };
}

function sessionRows(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll('[data-testid="workflow-session-row"]');
}

describe('WorkflowView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the unfiltered session list when filterWorkflowId is null', () => {
    const { wrapper } = mountWorkflowView(null);
    const rows = sessionRows(wrapper);

    expect(rows).toHaveLength(3);
    expect(wrapper.text()).toContain('active');
    expect(wrapper.text()).toContain('completed');
    expect(wrapper.text()).toContain('cancelled');
  });

  it('renders only sessions whose workflow_id matches filterWorkflowId', () => {
    const { wrapper } = mountWorkflowView('workflow-A');
    const rows = sessionRows(wrapper);

    expect(rows).toHaveLength(2);
    expect(wrapper.text()).toContain('workflow-A');
    expect(wrapper.text()).toContain('active');
    expect(wrapper.text()).toContain('cancelled');
    expect(wrapper.text()).not.toContain('workflow-B');
    expect(wrapper.text()).not.toContain('completed');
  });

  it('updates the rendered session list when filterWorkflowId changes', async () => {
    const { wrapper } = mountWorkflowView('workflow-A');

    await wrapper.setProps({ filterWorkflowId: 'workflow-B' });

    const rows = sessionRows(wrapper);
    expect(rows).toHaveLength(1);
    expect(wrapper.text()).toContain('workflow-B');
    expect(wrapper.text()).toContain('completed');
    expect(wrapper.text()).not.toContain('workflow-A');
    expect(wrapper.text()).not.toContain('cancelled');
  });

  it('keeps session row clicks wired to detail selection', async () => {
    const { store, wrapper } = mountWorkflowView('workflow-B');
    const selectSession = vi.spyOn(store, 'selectSession').mockResolvedValue();

    await sessionRows(wrapper)[0].trigger('click');

    expect(selectSession).toHaveBeenCalledWith('session-b1');
  });
});
