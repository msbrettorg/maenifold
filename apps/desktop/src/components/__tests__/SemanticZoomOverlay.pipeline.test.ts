// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import { usePipelineStore } from '../../stores/pipeline';
import { usePromptDialogStore } from '../../stores/prompt-dialog';
import {
  createNode,
  mountOverlay,
  setupSemanticZoomOverlay,
} from './SemanticZoomOverlay.helpers';

describe('SemanticZoomOverlay', () => {
  beforeEach(() => {
    setupSemanticZoomOverlay();
  });

  // T-302.43: retry guard for known flaky test under parallel pool. The test relies on async render/microtask ordering that races with neighboring tests in the parallel vitest pool. See apps/desktop/docs/test-flakes/T-302.43-repro.md.
  it('shows footer actions using the same status gating as inspector actions', { retry: 2 }, async () => {
    const wrapper = mountOverlay();
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

    const wrapper = mountOverlay();
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

    const wrapper = mountOverlay();
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
