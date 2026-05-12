// @vitest-environment jsdom
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';

import PipelineNodeCard from '../PipelineNodeCard.vue';

const storeMock = vi.hoisted(() => ({
  resumeFromNode: vi.fn(async () => {}),
  currentView: { running: false },
}));

vi.mock('../../stores/pipeline', () => ({
  usePipelineStore: () => storeMock,
}));

function mountCard(status: 'abandoned' | 'stuck' | 'completed') {
  return mount(PipelineNodeCard, {
    props: {
      id: 'failed-node',
      data: {
        nodeId: 'failed-node',
        displayLabel: 'Failed node',
        status,
        attempts: 1,
        type: 'tool',
        toolName: 'example.tool',
      },
    },
    global: {
      stubs: {
        PipelineNodeCardPorts: true,
      },
    },
  });
}

describe('PipelineNodeCard resume action', () => {
  beforeEach(() => {
    storeMock.currentView.running = false;
    storeMock.resumeFromNode.mockClear();
  });

  it('shows a resume-from-node button for abandoned nodes and invokes the store action', async () => {
    const wrapper = mountCard('abandoned');
    const button = wrapper.get('button[aria-label="Resume pipeline from this node"]');

    expect(button.text()).toContain('Resume from this node');

    await button.trigger('click');
    await flushPromises();

    expect(storeMock.resumeFromNode).toHaveBeenCalledWith('failed-node');
  });

  it('shows the same resume action for stuck nodes', () => {
    const wrapper = mountCard('stuck');

    expect(wrapper.find('button[aria-label="Resume pipeline from this node"]').exists()).toBe(true);
  });

  it('does not show the resume action for completed nodes', () => {
    const wrapper = mountCard('completed');

    expect(wrapper.find('button[aria-label="Resume pipeline from this node"]').exists()).toBe(false);
  });
});
