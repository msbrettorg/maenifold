import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

// Mock the gateway boundary the store talks to. Every store action funnels
// through `executeGateway` (the GatewayExecute command surface) or
// `readGatewayResource` (the `pipeline://sessions` MCP resource); mocking the
// module lets us assert the exact domain/command/parameter shape each action
// sends without touching the real MCP transport.
vi.mock('../lib/mcp/gateway', () => ({
  executeGateway: vi.fn(),
  readGatewayResource: vi.fn(),
}));

import { executeGateway, readGatewayResource } from '../lib/mcp/gateway';
import { usePipelinesStore } from './pipelines';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
} from '../lib/pipeline-types';

const executeGatewayMock = vi.mocked(executeGateway);
const readGatewayResourceMock = vi.mocked(readGatewayResource);

const WORKFLOW_DOMAIN = 'maenifold.workflow';
const PIPELINE_COMMAND = 'Pipeline';

// A minimal-but-valid definition: one prompt node, one edge carrying the
// REQUIRED `type` discriminator the gateway schema demands.
function sampleDefinition(): PipelineDefinition {
  const node: PipelineNode = {
    id: 'n1',
    type: 'prompt',
    prompt: 'do the thing',
    agent: 'Copilot CLI',
  };
  const edge: PipelineEdge = { source: 'n1', target: 'n2', type: 'success' };
  return { nodes: [node], edges: [edge] };
}

beforeEach(() => {
  setActivePinia(createPinia());
  executeGatewayMock.mockReset();
  readGatewayResourceMock.mockReset();
  // Default: resolve so actions that don't care about the return value settle.
  executeGatewayMock.mockResolvedValue(undefined as unknown as never);
  readGatewayResourceMock.mockResolvedValue(undefined as unknown as never);
});

// Helper: assert the single executeGateway call hit the right domain/command
// and return the parameter object for shape assertions.
function lastParams(): Record<string, unknown> {
  expect(executeGatewayMock).toHaveBeenCalledTimes(1);
  const [domain, command, params] = executeGatewayMock.mock.calls[0];
  expect(domain).toBe(WORKFLOW_DOMAIN);
  expect(command).toBe(PIPELINE_COMMAND);
  return params as Record<string, unknown>;
}

describe('usePipelinesStore — action → gateway op mapping', () => {
  it('createPipeline sends definition + name + description + maxConcurrency default', async () => {
    const store = usePipelinesStore();
    const definition = sampleDefinition();
    await store.createPipeline(definition, { name: '  My Pipe  ', description: '  desc  ' });

    const params = lastParams();
    expect(params).toEqual({
      definition,
      maxConcurrency: 8,
      name: 'My Pipe',
      description: 'desc',
    });
  });

  it('createPipeline omits name/description when blank and honors explicit maxConcurrency', async () => {
    const store = usePipelinesStore();
    const definition = sampleDefinition();
    await store.createPipeline(definition, { maxConcurrency: 3, name: '   ' });

    const params = lastParams();
    expect(params).toEqual({ definition, maxConcurrency: 3 });
    expect('name' in params).toBe(false);
    expect('description' in params).toBe(false);
  });

  it('updatePipeline sets update:true with sessionId + definition', async () => {
    const store = usePipelinesStore();
    const definition = sampleDefinition();
    await store.updatePipeline('sess-1', definition, { name: 'Renamed' });

    const params = lastParams();
    expect(params).toEqual({
      sessionId: 'sess-1',
      definition,
      name: 'Renamed',
      update: true,
    });
  });

  it('runPipeline sends run:true with trimmed prompt + non-empty inputs', async () => {
    const store = usePipelinesStore();
    await store.runPipeline('sess-1', { prompt: '  go  ', inputs: { foo: 'bar' } });

    const params = lastParams();
    expect(params).toEqual({
      sessionId: 'sess-1',
      run: true,
      prompt: 'go',
      inputs: { foo: 'bar' },
    });
  });

  it('runPipeline omits prompt + empty inputs', async () => {
    const store = usePipelinesStore();
    await store.runPipeline('sess-1', { inputs: {} });

    const params = lastParams();
    expect(params).toEqual({ sessionId: 'sess-1', run: true });
    expect('prompt' in params).toBe(false);
    expect('inputs' in params).toBe(false);
  });

  it('pausePipeline sends pause:true', async () => {
    const store = usePipelinesStore();
    await store.pausePipeline('sess-1');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', pause: true });
  });

  it('resumePipeline sends resume:true', async () => {
    const store = usePipelinesStore();
    await store.resumePipeline('sess-1');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', resume: true });
  });

  it('cancelPipeline sends cancel:true', async () => {
    const store = usePipelinesStore();
    await store.cancelPipeline('sess-1');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', cancel: true });
  });

  it('dispatchPipeline sends dispatch:true', async () => {
    const store = usePipelinesStore();
    await store.dispatchPipeline('sess-1');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', dispatch: true });
  });

  it('deletePipeline sends delete:true and includes force only when requested', async () => {
    const store = usePipelinesStore();

    await store.deletePipeline('sess-1');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', delete: true });

    executeGatewayMock.mockClear();
    await store.deletePipeline('sess-2', true);
    expect(lastParams()).toEqual({ sessionId: 'sess-2', delete: true, force: true });
  });

  it('rewindNodes sends rewind:{nodeIds}', async () => {
    const store = usePipelinesStore();
    await store.rewindNodes('sess-1', ['a', 'b']);
    expect(lastParams()).toEqual({ sessionId: 'sess-1', rewind: { nodeIds: ['a', 'b'] } });
  });

  it('graftNodes sends graft:{nodes,edges}', async () => {
    const store = usePipelinesStore();
    const nodes = sampleDefinition().nodes;
    const edges: PipelineEdge[] = [{ source: 'n1', target: 'n2', type: 'always' }];
    await store.graftNodes('sess-1', nodes, edges);
    expect(lastParams()).toEqual({ sessionId: 'sess-1', graft: { nodes, edges } });
  });

  it('joinPipeline sends join:{sourceSessionId,edges}', async () => {
    const store = usePipelinesStore();
    const edges: PipelineEdge[] = [{ source: 'x', target: 'y', type: 'conditional' }];
    await store.joinPipeline('sess-1', 'sess-src', edges);
    expect(lastParams()).toEqual({
      sessionId: 'sess-1',
      join: { sourceSessionId: 'sess-src', edges },
    });
  });

  it('forkPipeline sends bare fromNodeId when no newSessionId', async () => {
    const store = usePipelinesStore();
    await store.forkPipeline('sess-1', 'node-7');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', fork: 'node-7' });
  });

  it('forkPipeline sends fork:{fromNodeId,newSessionId} when newSessionId provided', async () => {
    const store = usePipelinesStore();
    await store.forkPipeline('sess-1', 'node-7', 'sess-new');
    expect(lastParams()).toEqual({
      sessionId: 'sess-1',
      fork: { fromNodeId: 'node-7', newSessionId: 'sess-new' },
    });
  });

  it('restartNode sends nodeId + restart:true', async () => {
    const store = usePipelinesStore();
    await store.restartNode('sess-1', 'node-3');
    expect(lastParams()).toEqual({ sessionId: 'sess-1', nodeId: 'node-3', restart: true });
  });

  it('setNodeStatus sends nodeId + status (+ trimmed conclusion when present)', async () => {
    const store = usePipelinesStore();

    await store.setNodeStatus('sess-1', 'node-3', 'completed', '  done well  ');
    expect(lastParams()).toEqual({
      sessionId: 'sess-1',
      nodeId: 'node-3',
      status: 'completed',
      conclusion: 'done well',
    });

    executeGatewayMock.mockClear();
    await store.setNodeStatus('sess-1', 'node-4', 'abandoned');
    expect(lastParams()).toEqual({
      sessionId: 'sess-1',
      nodeId: 'node-4',
      status: 'abandoned',
    });
  });

  it('listPipelines sends list:true and normalizes the response', async () => {
    executeGatewayMock.mockResolvedValueOnce({
      pipelines: [
        { pipelineId: 'p1', status: 'running' },
        { notAPipeline: true },
      ],
    } as never);
    const store = usePipelinesStore();
    const items = await store.listPipelines();

    expect(lastParams()).toEqual({ list: true });
    expect(items).toHaveLength(1);
    expect(items[0].pipelineId).toBe('p1');
  });

  it('viewPipeline sends view:true and stores the normalized view', async () => {
    executeGatewayMock.mockResolvedValueOnce({
      pipelineId: 'p1',
      status: 'running',
      nodes: [],
      edges: [],
    } as never);
    const store = usePipelinesStore();
    const view = await store.viewPipeline('sess-1');

    expect(lastParams()).toEqual({ sessionId: 'sess-1', view: true });
    expect(view.pipelineId).toBe('p1');
    expect(store.selectedView?.pipelineId).toBe('p1');
    expect(store.selectedPipelineId).toBe('p1');
  });
});

describe('usePipelinesStore — view-payload normalizer (snake_case → camelCase)', () => {
  it('renames cognitive surfaces on the view nodes and populates the typed PipelineView', async () => {
    const raw = {
      pipelineId: 'p1',
      name: 'Demo',
      description: 'A demo pipe',
      status: 'active',
      final: false,
      running: true,
      mermaid: 'graph TD;',
      ready: ['n2'],
      active: ['n1'],
      completed: ['n0'],
      inputs: { foo: 'bar' },
      inputSchema: { type: 'object' },
      edges: [{ source: 'n0', target: 'n1', type: 'success' }],
      nodes: [
        {
          nodeId: 'n1',
          sessionId: 's1',
          status: 'active',
          attempts: 1,
          type: 'workflow',
          workflowId: 'wf',
          assembled_prompt: 'ASSEMBLED',
          workflow_steps: [
            { step_name: 'plan', status: 'completed', output: 'ok' },
          ],
        },
        {
          nodeId: 'n2',
          sessionId: 's2',
          status: 'pending',
          attempts: 0,
          type: 'sequentialThinking',
          topic: 'why',
          agent: 'Copilot CLI',
          assembled_prompt: 'THINK',
          thinking_trace: [
            {
              thought_number: 1,
              total_thoughts: 3,
              text: 'first',
              is_revision: false,
              revises_thought: null,
              branch_id: 'main',
            },
          ],
        },
      ],
    };

    executeGatewayMock.mockResolvedValueOnce(raw as never);
    const store = usePipelinesStore();
    const view = await store.viewPipeline('sess-1');

    // Top-level scalar fields carried through.
    expect(view.pipelineId).toBe('p1');
    expect(view.name).toBe('Demo');
    expect(view.description).toBe('A demo pipe');
    expect(view.status).toBe('active');
    expect(view.final).toBe(false);
    expect(view.running).toBe(true);
    expect(view.mermaid).toBe('graph TD;');
    expect(view.ready).toEqual(['n2']);
    expect(view.active).toEqual(['n1']);
    expect(view.completed).toEqual(['n0']);
    expect(view.inputs).toEqual({ foo: 'bar' });
    expect(view.inputSchema).toEqual({ type: 'object' });
    expect(view.edges).toEqual([{ source: 'n0', target: 'n1', type: 'success' }]);

    // Node 1: assembled_prompt → assembledPrompt; workflow_steps[].step_name → stepName.
    const node1 = view.nodes[0] as Record<string, unknown>;
    expect(node1.assembledPrompt).toBe('ASSEMBLED');
    expect('assembled_prompt' in node1).toBe(false);
    const steps = node1.workflowSteps as Array<Record<string, unknown>>;
    expect(steps[0].stepName).toBe('plan');
    expect('step_name' in steps[0]).toBe(false);
    expect(steps[0].status).toBe('completed');
    expect('workflow_steps' in node1).toBe(false);

    // Node 2: thinking_trace member fields renamed to camelCase.
    const node2 = view.nodes[1] as Record<string, unknown>;
    expect(node2.assembledPrompt).toBe('THINK');
    const trace = node2.thinkingTrace as Array<Record<string, unknown>>;
    expect(trace[0].thoughtNumber).toBe(1);
    expect(trace[0].totalThoughts).toBe(3);
    expect(trace[0].isRevision).toBe(false);
    expect(trace[0].revisesThought).toBe(null);
    expect(trace[0].branchId).toBe('main');
    expect('thought_number' in trace[0]).toBe(false);
    expect('thinking_trace' in node2).toBe(false);
  });

  it('throws when the view payload is not an object', async () => {
    executeGatewayMock.mockResolvedValueOnce('not-an-object' as never);
    const store = usePipelinesStore();
    await expect(store.viewPipeline('sess-1')).rejects.toThrow(
      /non-object payload/
    );
    expect(store.error).toMatch(/non-object payload/);
  });

  it('defaults missing/optional fields safely', async () => {
    executeGatewayMock.mockResolvedValueOnce({ status: 'pending' } as never);
    const store = usePipelinesStore();
    const view = await store.viewPipeline('sess-1');

    expect(view.pipelineId).toBe('');
    expect(view.name).toBe(null);
    expect(view.nodes).toEqual([]);
    expect(view.edges).toEqual([]);
    expect(view.ready).toEqual([]);
    expect(view.running).toBe(false);
    // The store does `view.pipelineId ?? sessionId`; the normalizer yields an
    // empty string (not null/undefined) for a missing pipelineId, so `??` does
    // NOT fall back and selectedPipelineId is the empty string.
    expect(store.selectedPipelineId).toBe('');
  });
});

describe('usePipelinesStore — refreshList against pipeline://sessions resource', () => {
  it('reads the resource and normalizes list items', async () => {
    readGatewayResourceMock.mockResolvedValueOnce({
      sessions: [
        { pipelineId: 'p1', status: 'running', nodeCount: 2, completedCount: 1 },
        { pipelineId: 'p2', status: 'completed', nodeCount: 1, completedCount: 1 },
        { missingId: true },
      ],
    } as never);

    const store = usePipelinesStore();
    const items = await store.refreshList();

    expect(readGatewayResourceMock).toHaveBeenCalledWith('pipeline://sessions');
    expect(items).toHaveLength(2);
    expect(items.map((p) => p.pipelineId)).toEqual(['p1', 'p2']);
    expect(store.pipelines).toHaveLength(2);
    expect(store.pipelineCount).toBe(2);
    expect(store.error).toBe(null);
  });

  it('surfaces errors and rethrows', async () => {
    readGatewayResourceMock.mockRejectedValueOnce(new Error('resource boom'));
    const store = usePipelinesStore();
    await expect(store.refreshList()).rejects.toThrow('resource boom');
    expect(store.error).toBe('resource boom');
    expect(store.loading).toBe(false);
  });
});
