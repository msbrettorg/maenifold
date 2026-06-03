import { describe, expect, it } from 'vitest';
import {
  copyDefinitionEdge,
  definitionToView,
  flowEdgeIds,
  indexForFlowEdgeId,
  viewToDefinition,
  type DesignViewMeta,
} from './pipelineDefinitionProjection';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineView,
  ViewNode,
} from './pipeline-types';

// A definition that exercises all four node-definition variants plus a
// conditional edge and a binding edge (with schemas on both endpoints).
function sampleDefinition(): PipelineDefinition {
  return {
    nodes: [
      {
        // WorkflowNode (type omitted — `type?: 'workflow'` is optional, and the
        // projection must preserve its absence).
        id: 'plan',
        workflowId: 'wf-plan',
        prompt: 'Plan the work',
        joinPolicy: 'all',
        retryMaxAttempts: 2,
        outputSchema: { type: 'object', properties: { summary: { type: 'string' } } },
      },
      {
        id: 'draft',
        type: 'prompt',
        prompt: 'Draft using {{summary}}',
        agent: 'copilot',
        model: 'gpt-5',
        allowedTools: ['read', 'write'],
        inputSchema: { type: 'object', properties: { summary: { type: 'string' } } },
        outputSchema: { type: 'object', properties: { draft: { type: 'string' } } },
      },
      {
        id: 'think',
        type: 'sequentialThinking',
        topic: 'Evaluate the draft',
        agent: 'copilot',
        totalThoughts: 5,
        adopt: { role: 'critic', color: 'black' },
      },
      {
        id: 'finalize',
        type: 'tool',
        toolName: 'ma_WriteMemory',
        toolArgs: { path: 'memory://draft' },
        inputSchema: { type: 'object', properties: { draft: { type: 'string' } } },
      },
    ],
    edges: [
      { source: 'plan', target: 'draft', type: 'success', mode: 'binding',
        bindings: [{ from: 'summary', to: 'summary' }] },
      { source: 'draft', target: 'think', type: 'always', mode: 'context' },
      { source: 'think', target: 'finalize', type: 'conditional', path: 'verdict',
        equals: 'approved', mode: 'binding',
        bindings: [{ from: 'draft', to: 'draft' }] },
    ],
    inputs: { goal: 'ship it' },
    inputSchema: { type: 'object', properties: { goal: { type: 'string' } } },
    templateId: 'tmpl-42',
  };
}

const META: DesignViewMeta = {
  pipelineId: 'pipe-1',
  name: 'Sample',
  description: 'desc',
  status: 'pending',
};

describe('pipelineDefinitionProjection round-trip', () => {
  it('definition -> view -> definition is lossless', () => {
    const original = sampleDefinition();
    const view = definitionToView(original, META);
    const restored = viewToDefinition(view);
    expect(restored).toEqual(original);
  });

  it('view -> definition drops runtime-only fields but keeps config', () => {
    const runtimeNode: ViewNode = {
      nodeId: 'plan',
      sessionId: 'sess-xyz',
      status: 'completed',
      attempts: 3,
      output: { summary: 'done' },
      startedAt: '2024-01-01T00:00:00Z',
      // configured fields that must survive:
      workflowId: 'wf-plan',
      prompt: 'Plan the work',
      outputSchema: { type: 'object', properties: { summary: { type: 'string' } } },
    } as unknown as ViewNode;

    const view: PipelineView = {
      pipelineId: 'pipe-1',
      name: 'Sample',
      description: null,
      status: 'pending',
      nodes: [runtimeNode],
      edges: [],
      ready: [],
      active: [],
      completed: [],
      mermaid: '',
      running: false,
    } as unknown as PipelineView;

    const def = viewToDefinition(view);
    const node = def.nodes[0] as unknown as Record<string, unknown>;
    expect(node.id).toBe('plan');
    expect(node.workflowId).toBe('wf-plan');
    expect(node.prompt).toBe('Plan the work');
    expect(node.outputSchema).toBeDefined();
    // runtime-only keys must be gone:
    expect(node.nodeId).toBeUndefined();
    expect(node.sessionId).toBeUndefined();
    expect(node.status).toBeUndefined();
    expect(node.attempts).toBeUndefined();
    expect(node.output).toBeUndefined();
    expect(node.startedAt).toBeUndefined();
  });
});

// Blue-team Defect B: the daemon view emits `mapping: null` (and may null other
// optional edge fields). A literal `null` fails the backend schema, so the
// projection must OMIT the key entirely, not pass `null` through.
describe('copyDefinitionEdge null handling', () => {
  it('omits mapping when the daemon emits mapping: null', () => {
    const edge = {
      source: 'a',
      target: 'b',
      type: 'success',
      mapping: null,
    } as unknown as PipelineEdge;
    const out = copyDefinitionEdge(edge) as unknown as Record<string, unknown>;
    expect(out).not.toHaveProperty('mapping');
    expect('mapping' in out).toBe(false);
    expect(out.source).toBe('a');
    expect(out.target).toBe('b');
    expect(out.type).toBe('success');
  });

  it('omits all nulled optional edge fields', () => {
    const edge = {
      source: 'a',
      target: 'b',
      type: 'always',
      path: null,
      equals: null,
      bindings: null,
      mapping: null,
    } as unknown as PipelineEdge;
    const out = copyDefinitionEdge(edge) as unknown as Record<string, unknown>;
    expect(out).not.toHaveProperty('path');
    expect(out).not.toHaveProperty('equals');
    expect(out).not.toHaveProperty('bindings');
    expect(out).not.toHaveProperty('mapping');
  });

  it('view -> definition strips mapping: null across all edges', () => {
    const view: PipelineView = {
      pipelineId: 'pipe-1',
      name: 'Sample',
      description: null,
      status: 'pending',
      nodes: [],
      edges: [
        { source: 'a', target: 'b', type: 'success', mapping: null },
        { source: 'b', target: 'c', type: 'always', mapping: null },
      ],
      ready: [],
      active: [],
      completed: [],
      mermaid: '',
      running: false,
    } as unknown as PipelineView;
    const def = viewToDefinition(view);
    for (const edge of def.edges) {
      expect(edge).not.toHaveProperty('mapping');
    }
  });
});

describe('flowEdgeIds / indexForFlowEdgeId', () => {
  it('assigns per-source indices and resolves parallel edges back to array index', () => {
    const def = sampleDefinition();
    // Add a parallel duplicate (plan -> draft) edge to exercise the counter.
    def.edges.push({ source: 'plan', target: 'draft', type: 'always' });
    const ids = flowEdgeIds(def.edges);
    expect(ids[0]).toBe('plan->draft#0');
    expect(ids[ids.length - 1]).toBe('plan->draft#1');
    expect(indexForFlowEdgeId(def.edges, 'plan->draft#1')).toBe(def.edges.length - 1);
    expect(indexForFlowEdgeId(def.edges, 'does->not#9')).toBe(-1);
  });
});
