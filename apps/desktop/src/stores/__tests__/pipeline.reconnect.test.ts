// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

import { usePipelineStore } from '../pipeline';
import type { PipelineView, ViewNode } from '../../lib/pipeline-types';

type ReconnectApi = {
  markNodeReconnected: (nodeId: string) => void;
  clearNodeReconnected: (nodeId: string) => void;
  replaceNodeOutput: (
    nodeId: string,
    output: string,
    metadata: { sessionId: string; takenAt: number },
  ) => void;
};

function node(nodeId: string, overrides: Partial<ViewNode> = {}): ViewNode {
  return {
    nodeId,
    sessionId: `session-${nodeId}`,
    status: 'completed',
    attempts: 1,
    output: `stored ${nodeId}`,
    workflowId: `workflow.${nodeId}`,
    ...overrides,
  } as ViewNode;
}

function view(nodes: ViewNode[]): PipelineView {
  return {
    pipelineId: 'pipe-reconnect-dirty',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: nodes.map((candidate) => candidate.nodeId),
    mermaid: 'graph TD;',
    nodes,
    edges: [
      { source: 'node-a', target: 'node-b', type: 'success' },
      { source: 'node-b', target: 'node-c', type: 'success' },
    ],
  };
}

describe('pipeline store reconnect dirty tracking', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('marks and clears a non-tool node reconnected flag in the current view', () => {
    const store = usePipelineStore();
    store.currentView = view([node('node-a')]);
    const api = store as unknown as ReconnectApi;

    expect(typeof api.markNodeReconnected).toBe('function');
    expect(typeof api.clearNodeReconnected).toBe('function');

    api.markNodeReconnected('node-a');
    expect((store.currentView!.nodes[0] as ViewNode & { reconnected?: boolean }).reconnected).toBe(true);

    api.clearNodeReconnected('node-a');
    expect((store.currentView!.nodes[0] as ViewNode & { reconnected?: boolean }).reconnected).toBe(false);
  });

  it('replaces stored output, clears reconnected, and marks all downstream nodes stale', () => {
    const store = usePipelineStore();
    store.currentView = view([
      node('node-a', { output: 'original output' } as Partial<ViewNode>),
      node('node-b'),
      node('node-c'),
    ]);
    const api = store as unknown as ReconnectApi;

    api.markNodeReconnected('node-a');
    api.replaceNodeOutput('node-a', 'refined output', {
      sessionId: 'acp-remote-1',
      takenAt: 1_777_000_000_000,
    });

    const [source, child, grandchild] = store.currentView!.nodes as Array<ViewNode & {
      reconnected?: boolean;
      stale?: boolean;
      refinement?: { sessionId: string; takenAt: number };
    }>;
    expect(source.output).toBe('refined output');
    expect(source.reconnected).toBe(false);
    expect(source.refinement).toEqual({
      sessionId: 'acp-remote-1',
      takenAt: 1_777_000_000_000,
    });
    expect(child.stale).toBe(true);
    expect(grandchild.stale).toBe(true);
  });

  it('does not dirty-track tool nodes', () => {
    const store = usePipelineStore();
    store.currentView = view([
      node('tool-node', { type: 'tool', toolName: 'example.echo' } as Partial<ViewNode>),
    ]);
    const api = store as unknown as ReconnectApi;

    api.markNodeReconnected('tool-node');

    expect((store.currentView!.nodes[0] as ViewNode & { reconnected?: boolean }).reconnected).not.toBe(true);
  });
});
