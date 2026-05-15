// @vitest-environment jsdom
import { expect, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount } from '@vue/test-utils';
import SemanticZoomOverlay from '../SemanticZoomOverlay.vue';
import { usePipelineStore } from '../../stores/pipeline';
import { useSemanticZoomStore } from '../../stores/semantic-zoom';
import type { ViewNode } from '../../lib/pipeline-types';

export function createNode(overrides: Partial<ViewNode> = {}): ViewNode {
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

export function hugeNodeOutput(): string {
  return Array.from({ length: 200 }, (_, index) =>
    JSON.stringify({
      index,
      message: 'synthetic output line that forces inspector overflow',
      payload: 'x'.repeat(80),
    }),
  ).join('\n');
}

export function setCurrentNode(node: ViewNode): void {
  const pipeline = usePipelineStore();
  pipeline.currentView!.completed = node.status === 'completed' ? [node.nodeId] : [];
  pipeline.currentView!.active = node.status === 'active' ? [node.nodeId] : [];
  pipeline.currentView!.nodes = [node];
}

export function mountOverlay() {
  return mount(SemanticZoomOverlay, {
    attachTo: document.body,
  });
}

export function findButtonByLabel(label: string): HTMLButtonElement {
  const button = Array.from(document.body.querySelectorAll('button'))
    .find((btn) => btn.textContent?.includes(label));
  expect(button, `${label} button should be present`).toBeDefined();
  return button!;
}

export function findDialogContent(): HTMLElement {
  const dialogContent = document.body.querySelector<HTMLElement>('[data-slot="dialog-content"]');
  expect(dialogContent, 'dialog content should be present').not.toBeNull();
  return dialogContent!;
}

export function findScrollContainer(): HTMLElement {
  const scrollContainer = document.body.querySelector<HTMLElement>('[data-slot="scroll-area"]');
  expect(scrollContainer, 'scroll area root should be present').not.toBeNull();
  return scrollContainer!;
}

export function completedNodeWithSession(
  type: 'workflow' | 'prompt' | 'sequentialThinking' | 'tool',
  sessionId: string | null | undefined,
): ViewNode {
  const detail = sessionId === undefined ? {} : { sessionId };
  const provenance = {
    agent: type === 'tool' ? null : 'architect',
    inspectable: true,
    detail,
  } as unknown as ViewNode['provenance'];

  switch (type) {
    case 'workflow':
      return createNode({ provenance });
    case 'prompt':
      return createNode({
        type: 'prompt',
        prompt: 'Reconnect me',
        agent: 'architect',
        provenance,
      });
    case 'sequentialThinking':
      return createNode({
        type: 'sequentialThinking',
        topic: 'Reconnect me',
        agent: 'architect',
        provenance,
      });
    case 'tool':
      return createNode({
        type: 'tool',
        toolName: 'example.echo',
        provenance,
      });
  }
}

export function setupSemanticZoomOverlay(): void {
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
}
