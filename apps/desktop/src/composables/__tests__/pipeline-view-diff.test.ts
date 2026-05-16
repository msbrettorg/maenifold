import { describe, expect, it } from 'vitest';
import type { PipelineView } from '../../lib/pipeline-types-view';
import {
  buildNodeOutputPreview,
  hydrateEventLogFromView,
} from '../pipeline-view-diff';

function createView(): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: ['node-active'],
    completed: ['node-completed'],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'node-pending',
        sessionId: 'session-pending',
        status: 'pending',
        attempts: 0,
        type: 'tool',
        toolName: 'example.pending',
      },
      {
        nodeId: 'node-active',
        sessionId: 'session-active',
        status: 'active',
        attempts: 1,
        type: 'tool',
        toolName: 'example.active',
      },
      {
        nodeId: 'node-completed',
        sessionId: 'session-completed',
        status: 'completed',
        attempts: 1,
        type: 'tool',
        toolName: 'example.completed',
        output: '  alpha\nbeta  ',
      },
    ],
  };
}

describe('pipeline view diff hydration', () => {
  it('creates loaded entries only for non-pending nodes', () => {
    const entries = hydrateEventLogFromView(createView());

    expect(entries).toHaveLength(2);
    expect(entries.map(({ nodeId, from, to, preview }) => ({
      nodeId,
      from,
      to,
      preview,
    }))).toEqual([
      {
        nodeId: 'node-active',
        from: 'loaded',
        to: 'active',
        preview: undefined,
      },
      {
        nodeId: 'node-completed',
        from: 'loaded',
        to: 'completed',
        preview: 'alpha beta',
      },
    ]);
  });

  it('snapshots the completed preview at hydration time', () => {
    const view = createView();
    const [_, entry] = hydrateEventLogFromView(view);

    view.nodes[2]!.output = 'mutated after load';

    expect(entry?.preview).toBe(buildNodeOutputPreview({ output: '  alpha\nbeta  ' }));
    expect(entry?.preview).not.toBe(buildNodeOutputPreview(view.nodes[2]));
  });

  it('captures abandoned previews during hydration', () => {
    const view = createView();
    view.nodes[1] = {
      ...view.nodes[1]!,
      status: 'abandoned',
      output: '  tool execution failed  ',
    };

    const [entry] = hydrateEventLogFromView(view);

    expect(entry).toMatchObject({
      nodeId: 'node-active',
      from: 'loaded',
      to: 'abandoned',
      preview: 'tool execution failed',
    });
  });
});
