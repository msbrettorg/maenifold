import { flushPromises } from '@vue/test-utils';
import { computed } from 'vue';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { toast } from 'vue-sonner';
import { useComposerContextMenu } from '../useComposerContextMenu';
import { usePipelineStore } from '../../stores/pipeline';
import { usePromptDialogStore } from '../../stores/prompt-dialog';
import type { PipelineView } from '../../lib/pipeline-types-view';

const toastMock = vi.hoisted(() => ({
  error: vi.fn(),
}));

vi.mock('vue-sonner', () => ({ toast: toastMock }));

function createView(): PipelineView {
  return {
    pipelineId: 'session-1',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: ['node-1'],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [{
      nodeId: 'node-1',
      sessionId: 'session-1',
      status: 'completed',
      attempts: 1,
      output: 'node output',
      workflowId: 'workflow.alpha',
    }],
  } as PipelineView;
}

describe('useComposerContextMenu fork handling', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('shows a user-visible error toast when fork dispatch fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const pipeline = usePipelineStore();
    pipeline.currentPipelineId = 'session-1';
    pipeline.currentView = createView();
    pipeline.forkPipeline = vi.fn(async () => {
      throw new Error('transport unavailable');
    });

    const promptDialog = usePromptDialogStore();
    vi.spyOn(promptDialog, 'requestValue').mockResolvedValue('new-session');

    const contextMenu = useComposerContextMenu({
      designMode: () => false,
      view: () => pipeline.currentView,
      firstAgent: computed(() => 'architect'),
      firstWorkflow: computed(() => 'workflow.alpha'),
      createNodeAt: vi.fn(),
      editNode: vi.fn(),
      duplicateNode: vi.fn(),
      deleteNode: vi.fn(),
      disconnectNodeEdges: vi.fn(),
      toFlowPosition: vi.fn(() => ({ x: 0, y: 0 })),
      emitNodeDoubleClick: vi.fn(),
      emitOpenGraft: vi.fn(),
      emitOpenPromotion: vi.fn(),
    });

    contextMenu.contextMenu.value = { mode: 'node', nodeId: 'node-1' };
    contextMenu.handleServerNodeAction('forkNode');
    await flushPromises();

    expect(toast.error).toHaveBeenCalledWith('Fork failed — please retry', {
      closeButton: true,
      duration: Infinity,
    });
    expect(consoleError).toHaveBeenCalledWith('Fork pipeline session failed', expect.any(Error));
  });
});
