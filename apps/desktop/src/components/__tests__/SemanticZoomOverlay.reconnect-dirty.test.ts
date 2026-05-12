// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

import SemanticZoomOverlay from '../SemanticZoomOverlay.vue';
import { useAppShellStore } from '../../stores/app-shell';
import { usePipelineStore } from '../../stores/pipeline';
import { useSemanticZoomStore } from '../../stores/semantic-zoom';
import { useSessionStore } from '../../stores/session';
import type { PipelineView, ViewNode } from '../../lib/pipeline-types';

function node(overrides: Partial<ViewNode> = {}): ViewNode {
  return {
    nodeId: 'node-1',
    sessionId: 'pipeline-node-session',
    status: 'completed',
    attempts: 1,
    output: 'original stored output',
    workflowId: 'workflow.alpha',
    provenance: {
      agent: 'Copilot',
      inspectable: true,
      detail: { sessionId: 'acp-remote-1' },
    },
    ...overrides,
  } as ViewNode;
}

function setCurrentNode(viewNode: ViewNode): void {
  const store = usePipelineStore();
  store.currentView = {
    pipelineId: 'pipe-1',
    status: 'paused',
    running: false,
    ready: [],
    active: viewNode.status === 'active' ? [viewNode.nodeId] : [],
    completed: viewNode.status === 'completed' ? [viewNode.nodeId] : [],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [viewNode],
  } as PipelineView;
}

function setupOverlay(viewNode: ViewNode) {
  setActivePinia(createPinia());
  setCurrentNode(viewNode);
  const zoom = useSemanticZoomStore();
  zoom.level = 'inspect';
  zoom.focusedNodeId = viewNode.nodeId;
  const pipeline = usePipelineStore() as unknown as {
    replaceNodeOutput: ReturnType<typeof vi.fn>;
    clearNodeReconnected: ReturnType<typeof vi.fn>;
    markNodeReconnected: ReturnType<typeof vi.fn>;
  };
  pipeline.replaceNodeOutput = vi.fn();
  pipeline.clearNodeReconnected = vi.fn();
  pipeline.markNodeReconnected = vi.fn();
  return {
    wrapper: mount(SemanticZoomOverlay, { attachTo: document.body }),
    pipeline,
  };
}

function button(label: string): HTMLButtonElement {
  const found = Array.from(document.body.querySelectorAll('button'))
    .find((candidate) => candidate.textContent?.includes(label));
  expect(found, `${label} button should be present`).toBeDefined();
  return found!;
}

describe('SemanticZoomOverlay reconnect dirty controls', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    document.body.innerHTML = '';
  });

  it('shows take and discard refinement actions for completed reconnected non-tool nodes', async () => {
    const { wrapper } = setupOverlay(node({ reconnected: true } as Partial<ViewNode>));
    await flushPromises();

    expect(document.body.textContent).toContain('Fork from node');
    expect(document.body.textContent).toContain('Rewind node');
    expect(document.body.textContent).toContain('Graft from node');
    expect(document.body.textContent).toContain('Take refined output');
    expect(document.body.textContent).toContain('Discard refinement');

    wrapper.unmount();
  });

  it('hides take and discard refinement actions when the node is not reconnected', async () => {
    const { wrapper } = setupOverlay(node({ reconnected: false } as Partial<ViewNode>));
    await flushPromises();

    expect(document.body.textContent).not.toContain('Take refined output');
    expect(document.body.textContent).not.toContain('Discard refinement');

    wrapper.unmount();
  });

  it('hides reconnect and refinement actions for tool nodes regardless of dirty state', async () => {
    const { wrapper } = setupOverlay(node({
      type: 'tool',
      toolName: 'example.echo',
      reconnected: true,
    } as Partial<ViewNode>));
    await flushPromises();

    expect(document.body.textContent).not.toContain('Reconnect');
    expect(document.body.textContent).not.toContain('Take refined output');
    expect(document.body.textContent).not.toContain('Discard refinement');

    wrapper.unmount();
  });

  it('takes the latest assistant message from the resumed session into node output', async () => {
    const { wrapper, pipeline } = setupOverlay(node({ reconnected: true } as Partial<ViewNode>));
    const sessionStore = useSessionStore() as unknown as {
      activeSessionId: string;
      sessions: Map<string, unknown>;
    };
    sessionStore.activeSessionId = 'local-1';
    sessionStore.sessions = new Map([[
      'local-1',
      {
        sessionId: 'local-1',
        session: { sessionId: 'acp-remote-1' },
        messages: [
          { id: 'user-1', role: 'user', content: 'refine this', timestamp: 1 },
          { id: 'assistant-1', role: 'assistant', content: 'first answer', timestamp: 2 },
          { id: 'assistant-2', role: 'assistant', content: 'latest refined answer', timestamp: 3 },
        ],
      },
    ]]);
    await flushPromises();

    button('Take refined output').click();
    await flushPromises();

    expect(pipeline.replaceNodeOutput).toHaveBeenCalledWith(
      'node-1',
      'latest refined answer',
      expect.objectContaining({
        sessionId: 'acp-remote-1',
        takenAt: expect.any(Number),
      }),
    );
    wrapper.unmount();
  });

  it('discard refinement clears the reconnected flag without replacing stored output', async () => {
    const { wrapper, pipeline } = setupOverlay(node({ reconnected: true } as Partial<ViewNode>));
    await flushPromises();

    button('Discard refinement').click();
    await flushPromises();

    expect(pipeline.clearNodeReconnected).toHaveBeenCalledWith('node-1');
    expect(pipeline.replaceNodeOutput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('marks the node reconnected after a successful reconnect', async () => {
    const { wrapper, pipeline } = setupOverlay(node());
    const sessionStore = useSessionStore();
    const shell = useAppShellStore();
    vi.spyOn(sessionStore, 'focusOrResumeRemoteSession').mockResolvedValue({
      status: 'resumed',
      localSessionId: 'local-1',
    });
    vi.spyOn(shell, 'openAgents').mockImplementation(() => {});
    await flushPromises();

    button('Reconnect').click();
    await flushPromises();

    expect(sessionStore.focusOrResumeRemoteSession).toHaveBeenCalledWith(
      'acp-remote-1',
      'Copilot',
      { suppressErrors: true },
    );
    expect(pipeline.markNodeReconnected).toHaveBeenCalledWith('node-1');
    wrapper.unmount();
  });
});
