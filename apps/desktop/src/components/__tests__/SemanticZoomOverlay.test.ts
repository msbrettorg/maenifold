import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import SemanticZoomOverlay from '../SemanticZoomOverlay.vue';
import { usePipelineStore } from '../../stores/pipeline';
import { usePromptDialogStore } from '../../stores/prompt-dialog';
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

  it('shows footer actions using the same status gating as inspector actions', async () => {
    const wrapper = mount(SemanticZoomOverlay, {
      attachTo: document.body,
    });
    await flushPromises();

    expect(document.body.textContent).toContain('Fork from node');
    expect(document.body.textContent).toContain('Rewind node');
    expect(document.body.textContent).not.toContain('Restart node');
    expect(document.body.textContent).not.toContain('Abandon node');
    wrapper.unmount();
  });

  it('shows restart and rewind for abandoned nodes', async () => {
    const pipeline = usePipelineStore();
    pipeline.currentView!.nodes = [createNode({ status: 'abandoned' })];

    const wrapper = mount(SemanticZoomOverlay, {
      attachTo: document.body,
    });
    await flushPromises();

    expect(document.body.textContent).toContain('Fork from node');
    expect(document.body.textContent).toContain('Restart node');
    expect(document.body.textContent).toContain('Rewind node');
    expect(document.body.textContent).not.toContain('Abandon node');
    wrapper.unmount();
  });

  it('wires fork actions through the pipeline store and shows the returned session id', async () => {
    const pipeline = usePipelineStore();
    const promptDialog = usePromptDialogStore();
    const requestValueSpy = vi.spyOn(promptDialog, 'requestValue').mockResolvedValue('');

    const wrapper = mount(SemanticZoomOverlay, {
      attachTo: document.body,
    });
    await flushPromises();

    const forkButton = Array.from(document.body.querySelectorAll('button'))
      .find((btn) => btn.textContent?.trim() === 'Fork from node');
    expect(forkButton, 'Fork from node button should be present').toBeDefined();
    forkButton!.click();
    await flushPromises();

    expect(requestValueSpy).toHaveBeenCalledWith(expect.objectContaining({
      title: 'Fork pipeline session',
    }));
    expect(pipeline.forkPipeline).toHaveBeenCalledWith('session-1', 'node-1', undefined);
    expect(document.body.textContent).toContain('fork-1');

    requestValueSpy.mockRestore();
    wrapper.unmount();
  });
});
