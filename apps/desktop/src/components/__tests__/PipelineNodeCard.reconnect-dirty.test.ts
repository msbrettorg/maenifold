// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';

import PipelineNodeCard from '../PipelineNodeCard.vue';

vi.mock('../../stores/pipeline', () => ({
  usePipelineStore: () => ({
    resumeFromNode: vi.fn(async () => {}),
    currentView: { running: false },
  }),
}));

describe('PipelineNodeCard reconnect dirty badge', () => {
  it('renders a refining marker when a node has reconnected dirty output', () => {
    const wrapper = mount(PipelineNodeCard, {
      props: {
        id: 'node-1',
        data: {
          nodeId: 'node-1',
          displayLabel: 'Dirty node',
          status: 'completed',
          attempts: 1,
          workflowId: 'workflow.alpha',
          reconnected: true,
        } as never,
      },
      global: {
        stubs: {
          PipelineNodeCardPorts: true,
        },
      },
    });

    expect(wrapper.text()).toContain('↻ refining');
  });
});
