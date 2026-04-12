import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import SemanticZoomOverlay from '../SemanticZoomOverlay.vue';
import { usePipelineStore } from '../../stores/pipeline';
import { useSemanticZoomStore } from '../../stores/semantic-zoom';
import type { ViewNode } from '../../lib/pipeline-types';

function createNode(overrides: Partial<ViewNode> = {}): ViewNode {
  return {
    nodeId: 'node-1',
    sessionId: 'session-1',
    status: 'completed',
    attempts: 1,
    output: 'node output',
    workflowId: 'workflow.alpha',
    ...overrides,
  } as ViewNode;
}

describe('SemanticZoomOverlay', () => {
  beforeEach(() => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const pipeline = usePipelineStore();
    pipeline.currentView = {
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
    pipeline.restartNode = vi.fn(async () => undefined);
    pipeline.rewindNodes = vi.fn(async () => undefined);
    pipeline.updateNodeStatus = vi.fn(async () => undefined);
    pipeline.forkPipeline = vi.fn(async () => ({ sessionId: 'fork-1' }));

    const zoom = useSemanticZoomStore();
    zoom.level = 'inspect';
    zoom.focusedNodeId = 'node-1';
  });

  it('shows footer actions using the same status gating as inspector actions', () => {
    const wrapper = mount(SemanticZoomOverlay, {
      global: {
        stubs: {
          teleport: true,
        },
      },
    });

    expect(wrapper.text()).toContain('Fork from node');
    expect(wrapper.text()).toContain('Rewind node');
    expect(wrapper.text()).not.toContain('Restart node');
    expect(wrapper.text()).not.toContain('Abandon node');
  });

  it('shows restart and rewind for abandoned nodes', () => {
    const pipeline = usePipelineStore();
    pipeline.currentView!.nodes = [createNode({ status: 'abandoned' })];

    const wrapper = mount(SemanticZoomOverlay, {
      global: {
        stubs: {
          teleport: true,
        },
      },
    });

    expect(wrapper.text()).toContain('Fork from node');
    expect(wrapper.text()).toContain('Restart node');
    expect(wrapper.text()).toContain('Rewind node');
    expect(wrapper.text()).not.toContain('Abandon node');
  });

  it('wires fork actions through the pipeline store and shows the returned session id', async () => {
    const pipeline = usePipelineStore();
    const promptSpy = vi.spyOn(window, 'prompt').mockReturnValue('');

    const wrapper = mount(SemanticZoomOverlay, {
      global: {
        stubs: {
          teleport: true,
        },
      },
    });

    await wrapper.get('button.btn-ghost').trigger('click');
    await flushPromises();

    expect(promptSpy).toHaveBeenCalledWith('Optional name for the forked pipeline session:', '');
    expect(pipeline.forkPipeline).toHaveBeenCalledWith('session-1', 'node-1', undefined);
    expect(wrapper.text()).toContain('fork-1');

    promptSpy.mockRestore();
  });
});
