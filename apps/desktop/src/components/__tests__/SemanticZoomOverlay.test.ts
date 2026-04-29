// @vitest-environment jsdom
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

function hugeNodeOutput(): string {
  return Array.from({ length: 200 }, (_, index) =>
    JSON.stringify({
      index,
      message: 'synthetic output line that forces inspector overflow',
      payload: 'x'.repeat(80),
    }),
  ).join('\n');
}

function setCurrentNode(node: ViewNode): void {
  const pipeline = usePipelineStore();
  pipeline.currentView!.completed = node.status === 'completed' ? [node.nodeId] : [];
  pipeline.currentView!.active = node.status === 'active' ? [node.nodeId] : [];
  pipeline.currentView!.nodes = [node];
}

function mountOverlay() {
  return mount(SemanticZoomOverlay, {
    attachTo: document.body,
  });
}

function findButtonByLabel(label: string): HTMLButtonElement {
  const button = Array.from(document.body.querySelectorAll('button'))
    .find((btn) => btn.textContent?.includes(label));
  expect(button, `${label} button should be present`).toBeDefined();
  return button!;
}

function findDialogContent(): HTMLElement {
  const dialogContent = document.body.querySelector<HTMLElement>('[data-slot="dialog-content"]');
  expect(dialogContent, 'dialog content should be present').not.toBeNull();
  return dialogContent!;
}

function findScrollContainer(): HTMLElement {
  const scrollContainer = document.body.querySelector<HTMLElement>('[data-slot="scroll-area"]');
  expect(scrollContainer, 'scroll area root should be present').not.toBeNull();
  return scrollContainer!;
}

describe('SemanticZoomOverlay', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    document.body.innerHTML = '';

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

  // Visual scroll behavior is verified via CDP UAT, not vitest.
  // JSDOM does not compute layout; offsetHeight mocks are dishonest.
  // These tests assert the structural CSS contract only.
  it('puts the scroll contract on the ScrollArea flex child', async () => {
    setCurrentNode(createNode({ output: hugeNodeOutput() }));
    const wrapper = mountOverlay();
    await flushPromises();

    const scrollContainer = findScrollContainer();
    expect(scrollContainer.classList.contains('flex-1')).toBe(true);
    expect(scrollContainer.classList.contains('min-h-0')).toBe(true);
    expect(scrollContainer.classList.contains('overflow-y-auto')).toBe(true);

    wrapper.unmount();
  });

  it('keeps footer action buttons outside the scroll container', async () => {
    setCurrentNode(createNode({
      output: hugeNodeOutput(),
      provenance: {
        agent: 'swe',
        inspectable: true,
        detail: {
          canReconnect: true,
          sessionId: 'acp-session-1',
        },
      },
    }));
    const completedWrapper = mountOverlay();
    await flushPromises();

    const dialogContent = findDialogContent();
    const scrollContainer = findScrollContainer();
    const reconnectButton = findButtonByLabel('Reconnect');
    const footer = reconnectButton.closest('footer');

    expect(scrollContainer.parentElement).toBe(dialogContent);
    expect(footer, 'footer controls should be present').not.toBeNull();
    expect(footer?.parentElement).toBe(dialogContent);
    expect(reconnectButton.closest('[data-slot="scroll-area"]')).toBeNull();
    expect(Array.from(dialogContent.children)).toContain(scrollContainer);
    expect(Array.from(dialogContent.children)).toContain(footer);

    completedWrapper.unmount();
  });
});
