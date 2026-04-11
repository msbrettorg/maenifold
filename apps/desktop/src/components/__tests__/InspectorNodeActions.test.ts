import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount } from '@vue/test-utils';
import InspectorNodeActions from '../InspectorNodeActions.vue';
import { usePipelineStore } from '../../stores/pipeline';
import type { ViewNode } from '../../lib/pipeline-types';

vi.mock('../PromotionDialog.vue', () => ({
  default: {
    name: 'PromotionDialog',
    props: ['node', 'pipelineId'],
    emits: ['close', 'busy-change'],
    template: '<div class="promotion-dialog-stub">Promotion dialog</div>',
  },
}));

function createNode(overrides: Partial<ViewNode> = {}): ViewNode {
  return {
    nodeId: 'node-1',
    sessionId: 'session-1',
    status: 'completed',
    attempts: 1,
    output: 'useful output',
    workflowId: 'workflow.alpha',
    ...overrides,
  } as ViewNode;
}

describe('InspectorNodeActions', () => {
  beforeEach(() => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const store = usePipelineStore();
    store.currentView = {
      pipelineId: 'pipe-1',
      status: 'paused',
      running: false,
      ready: [],
      active: [],
      completed: ['node-1'],
      mermaid: 'graph TD;',
      edges: [],
      nodes: [createNode()],
    };
  });

  it('shows the promote button only for completed nodes with output', async () => {
    const wrapper = mount(InspectorNodeActions, {
      props: {
        node: createNode(),
        nodeId: 'node-1',
      },
    });

    expect(wrapper.text()).toContain('Promote to Memory');

    await wrapper.get('button.btn-primary').trigger('click');
    expect(wrapper.find('.promotion-dialog-stub').exists()).toBe(true);
  });

  it('hides the promote button when output is empty or node is not completed', () => {
    const noOutputWrapper = mount(InspectorNodeActions, {
      props: {
        node: createNode({ output: '   ' }),
        nodeId: 'node-1',
      },
    });
    expect(noOutputWrapper.text()).not.toContain('Promote to Memory');

    const activeWrapper = mount(InspectorNodeActions, {
      props: {
        node: createNode({ status: 'active' }),
        nodeId: 'node-1',
      },
    });
    expect(activeWrapper.text()).not.toContain('Promote to Memory');
  });
});
