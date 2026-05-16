// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import type {
  PipelineDefinition,
} from '../../lib/pipeline-types-edge';
import type {
  PipelineListItem,
  PipelineView,
  ViewNode,
} from '../../lib/pipeline-types-view';
import { useMcpClient } from '../useMcpClient';
import { usePipelineMcp } from '../usePipelineMcp';

interface PipelineCreateResult {
  pipelineId?: string;
}

interface PipelineLifecycleResult {
  status?: string;
  sessionId?: string;
}

interface PipelineListResult {
  pipelines?: PipelineListItem[];
}

interface ToolEnvelope<T> {
  success?: boolean;
  data?: T;
  error?: unknown;
}

interface MutableTask {
  mode?: 'run' | 'skip' | 'only' | 'todo';
}

interface MutableSuite extends MutableTask {
  tasks?: MutableTask[];
}

const BASE_URL = 'http://127.0.0.1:8080';
const MCP_URL = `${BASE_URL}/mcp`;
const PROBE_TIMEOUT_MS = 3000;
function readNpmLifecycleEvent(): string | undefined {
  const processLike = (globalThis as typeof globalThis & {
    process?: { env?: Record<string, string | undefined> };
  }).process;
  return processLike?.env?.npm_lifecycle_event;
}

const RUN_E2E = readNpmLifecycleEvent() === 'test:e2e';
const PREFERRED_WORKFLOW_ID = 'convergent-thinking';

let serverAvailable = false;
let currentClient: ReturnType<typeof useMcpClient> | null = null;
let currentPipelineMcp: ReturnType<typeof usePipelineMcp> | null = null;
const createdPipelineIds = new Set<string>();

async function fetchWithTimeout(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const controller = new AbortController();
  const timer = globalThis.setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);

  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } finally {
    globalThis.clearTimeout(timer);
  }
}

function skipSuite(suite: MutableSuite) {
  suite.mode = 'skip';
  suite.tasks?.forEach((task) => {
    task.mode = 'skip';
  });
}

function tryParseJson(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function unwrapToolData<T>(value: unknown): T {
  let current = value;

  for (let i = 0; i < 3; i += 1) {
    if (typeof current === 'string') {
      current = tryParseJson(current);
      continue;
    }

    if (typeof current === 'object' && current !== null && 'data' in current) {
      current = (current as { data: unknown }).data;
      continue;
    }

    break;
  }

  return current as T;
}

function makeViewNode(nodeId: string, status: ViewNode['status']): ViewNode {
  return { nodeId, workflowId: 'wf-1', sessionId: 'sess-1', status, attempts: 1 };
}

function makeView(nodes: ViewNode[], overrides: Partial<PipelineView> = {}): PipelineView {
  return {
    pipelineId: 'pipe-1',
    status: 'active',
    nodes,
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: true,
    ...overrides,
  };
}

async function probeServer(): Promise<{ available: boolean; reason?: string }> {
  let probeSessionId: string | null = null;

  try {
    const response = await fetchWithTimeout(MCP_URL, {
      method: 'POST',
      headers: {
        Accept: 'application/json, text/event-stream',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'initialize',
        id: 0,
        params: {
          protocolVersion: '2025-03-26',
          capabilities: {},
          clientInfo: { name: 'pipeline-ui-e2e-probe', version: '0.0.1' },
        },
      }),
    });

    probeSessionId = response.headers.get('Mcp-Session-Id');

    if (!response.ok) {
      return { available: false, reason: `Probe returned HTTP ${response.status}` };
    }

    return { available: true };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    return { available: false, reason: message };
  } finally {
    if (probeSessionId) {
      try {
        await fetchWithTimeout(MCP_URL, {
          method: 'DELETE',
          headers: {
            Accept: 'application/json, text/event-stream',
            'Mcp-Session-Id': probeSessionId,
          },
        });
      } catch {
        // Best effort cleanup for the probe session.
      }
    }
  }
}

async function connectClient() {
  currentPipelineMcp = null;
  currentClient = useMcpClient(BASE_URL);
  await currentClient.connect();
  return currentClient;
}

async function connectPipelineMcp() {
  currentClient = null;
  currentPipelineMcp = usePipelineMcp(BASE_URL);
  await currentPipelineMcp.connect();
  return currentPipelineMcp;
}

async function getWorkflowIds(mcp: ReturnType<typeof usePipelineMcp>): Promise<string[]> {
  const workflows = await mcp.listWorkflows();
  expect(workflows.length).toBeGreaterThan(0);
  return workflows;
}

async function pickWorkflowId(mcp: ReturnType<typeof usePipelineMcp>): Promise<string> {
  const workflows = await getWorkflowIds(mcp);
  return workflows.find((workflowId) => workflowId === PREFERRED_WORKFLOW_ID) ?? workflows[0];
}

function buildDefinition(workflowId: string, nodeCount = 2): PipelineDefinition {
  const nodes = Array.from({ length: nodeCount }, (_, index) => ({
    id: `node-${index + 1}`,
    workflowId,
  }));

  const edges = nodes.slice(0, -1).map((node, index) => ({
    source: node.id,
    target: nodes[index + 1].id,
    type: 'success' as const,
  }));

  return { nodes, edges };
}

async function createPipelineSession(
  mcp: ReturnType<typeof usePipelineMcp>,
  workflowId: string,
  nodeCount = 2,
): Promise<string> {
  const createResult = (await mcp.createPipeline(buildDefinition(workflowId, nodeCount), 1)) as PipelineCreateResult;
  expect(typeof createResult.pipelineId).toBe('string');
  expect(createResult.pipelineId?.length).toBeGreaterThan(0);

  const pipelineId = createResult.pipelineId as string;
  createdPipelineIds.add(pipelineId);
  return pipelineId;
}

async function delay(ms: number): Promise<void> {
  await new Promise((resolve) => globalThis.setTimeout(resolve, ms));
}

describe.skipIf(!RUN_E2E)('MCP live integration', () => {
  beforeAll(async ({}, suite) => {
    const probe = await probeServer();
    serverAvailable = probe.available;

    if (!probe.available) {
      console.warn(
        `[mcpIntegration] Skipping live MCP integration tests: ${probe.reason ?? 'localhost:8080 is unreachable'}`,
      );
      skipSuite(suite as MutableSuite);
    }
  });

  afterEach(async () => {
    if (serverAvailable) {
      for (const pipelineId of createdPipelineIds) {
        if (currentPipelineMcp?.connected.value) {
          try {
            await currentPipelineMcp.deletePipeline(pipelineId);
            continue;
          } catch {
            // Fall through and try raw MCP cleanup.
          }
        }

        if (currentClient?.sessionId.value) {
          try {
            await currentClient.callTool('Pipeline', { sessionId: pipelineId, delete: true });
          } catch {
            // Ignore cleanup failures so tests surface the original assertion.
          }
        }
      }

      if (currentPipelineMcp?.connected.value) {
        try {
          await currentPipelineMcp.disconnect();
        } catch {
          // Best-effort cleanup.
        }
      }

      if (currentClient?.sessionId.value) {
        try {
          await currentClient.disconnect();
        } catch {
          // Best-effort cleanup.
        }
      }
    }

    createdPipelineIds.clear();
    currentClient = null;
    currentPipelineMcp = null;
    vi.restoreAllMocks();
  });

  it('connects with initialize/initialized handshake and stores the session id', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const client = await connectClient();

    expect(client.connected.value).toBe(true);
    expect(client.sessionId.value).toEqual(expect.any(String));
    expect(client.error.value).toBeNull();
    expect(fetchSpy).toHaveBeenCalledTimes(2);

    const [initializeUrl, initializeOptions] = fetchSpy.mock.calls[0] as [string | URL, RequestInit];
    expect(String(initializeUrl)).toBe(MCP_URL);
    expect(initializeOptions.method).toBe('POST');

    const initializeBody = JSON.parse(String(initializeOptions.body)) as Record<string, unknown>;
    expect(initializeBody).toMatchObject({
      jsonrpc: '2.0',
      method: 'initialize',
    });

    const [, initializedOptions] = fetchSpy.mock.calls[1] as [string | URL, RequestInit];
    const initializedBody = JSON.parse(String(initializedOptions.body)) as Record<string, unknown>;
    expect(initializedBody).toMatchObject({
      jsonrpc: '2.0',
      method: 'notifications/initialized',
    });
    expect((initializedOptions.headers as Record<string, string>)['Mcp-Session-Id']).toBe(
      client.sessionId.value,
    );
  });

  it('disconnects with DELETE and clears client state', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const client = await connectClient();
    const activeSessionId = client.sessionId.value;

    await client.disconnect();

    expect(client.connected.value).toBe(false);
    expect(client.sessionId.value).toBeNull();
    expect(client.error.value).toBeNull();
    expect(fetchSpy).toHaveBeenCalledTimes(3);

    const [disconnectUrl, disconnectOptions] = fetchSpy.mock.calls[2] as [string | URL, RequestInit];
    expect(String(disconnectUrl)).toBe(MCP_URL);
    expect(disconnectOptions.method).toBe('DELETE');
    expect((disconnectOptions.headers as Record<string, string>)['Mcp-Session-Id']).toBe(activeSessionId);
  });

  it('sends tools/call JSON-RPC and parses SSE tool responses into JSON', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const client = await connectClient();
    const result = await client.callTool<ToolEnvelope<PipelineListResult>>('Pipeline', { list: true });

    expect(result.success).toBe(true);
    expect(result.error).toBeNull();
    expect(Array.isArray(result.data?.pipelines)).toBe(true);
    expect(client.error.value).toBeNull();

    const [, callOptions] = fetchSpy.mock.calls[2] as [string | URL, RequestInit];
    const body = JSON.parse(String(callOptions.body)) as Record<string, unknown>;
    expect(body).toMatchObject({
      jsonrpc: '2.0',
      method: 'tools/call',
      params: {
        name: 'Pipeline',
        arguments: { list: true },
      },
    });
  });

  it('lists tools from the live server and includes Pipeline', async () => {
    const client = await connectClient();
    const tools = await client.listTools();

    expect(Array.isArray(tools)).toBe(true);
    expect(tools.some((tool) => tool.name === 'Pipeline')).toBe(true);
    expect(client.error.value).toBeNull();
  });

  it('unwraps Pipeline { list: true } results into a pipelines object', async () => {
    const mcp = await connectPipelineMcp();
    const result = await mcp.listPipelines();

    expect(result).toEqual(expect.objectContaining({ pipelines: expect.any(Array) }));
    expect(mcp.error.value).toBeNull();
  });

  it('creates, views, lists, and deletes a pipeline', async () => {
    const mcp = await connectPipelineMcp();
    const workflowId = await pickWorkflowId(mcp);
    const pipelineId = await createPipelineSession(mcp, workflowId, 2);

    const pipelines = await mcp.listPipelines();
    expect(pipelines.pipelines?.some((pipeline) => pipeline.pipelineId === pipelineId)).toBe(true);

    const view = await mcp.viewPipeline(pipelineId);
    expect(view.pipelineId).toBe(pipelineId);
    expect(view.nodes).toHaveLength(2);
    expect(view.edges).toHaveLength(1);
    expect(view.status).toEqual(expect.any(String));

    await expect(mcp.deletePipeline(pipelineId)).resolves.not.toThrow();
    createdPipelineIds.delete(pipelineId);

    await expect(mcp.viewPipeline(pipelineId)).rejects.toThrow();
  });

  it('runs a pipeline, observes a running view, pauses it, and surfaces the live resume outcome', async () => {
    const mcp = await connectPipelineMcp();
    const workflowId = await pickWorkflowId(mcp);
    const runningPipelineId = await createPipelineSession(mcp, workflowId, 1);

    const runResult = (await mcp.runPipeline(runningPipelineId)) as PipelineLifecycleResult;
    expect(runResult).toMatchObject({ status: 'started', sessionId: runningPipelineId });

    const runningView = await mcp.viewPipeline(runningPipelineId);
    expect(runningView.pipelineId).toBe(runningPipelineId);
    expect(runningView.running).toBe(true);

    const pausedPipelineId = await createPipelineSession(mcp, workflowId, 3);
    const pausedRunResult = (await mcp.runPipeline(pausedPipelineId)) as PipelineLifecycleResult;
    expect(pausedRunResult).toMatchObject({ status: 'started', sessionId: pausedPipelineId });

    const pauseResult = (await mcp.pausePipeline(pausedPipelineId)) as PipelineLifecycleResult;
    expect(pauseResult).toMatchObject({ status: 'paused', sessionId: pausedPipelineId });

    const pausedView = await mcp.viewPipeline(pausedPipelineId);
    expect(pausedView.pipelineId).toBe(pausedPipelineId);
    expect(pausedView.running).toBe(false);

    try {
      const resumeResult = (await mcp.resumePipeline(pausedPipelineId)) as PipelineLifecycleResult;
      expect(resumeResult).toMatchObject({ status: 'resumed', sessionId: pausedPipelineId });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      expect(message).toMatch(/no longer running|not paused/);
      expect(mcp.error.value).toBe(message);
    }
  });

  it('lists workflow ids via ListAssets and unwraps the nested response envelope', async () => {
    const mcp = await connectPipelineMcp();
    const workflows = await mcp.listWorkflows();

    expect(workflows.length).toBeGreaterThan(0);
    expect(workflows.every((workflowId) => typeof workflowId === 'string' && workflowId.length > 0)).toBe(true);
    expect(workflows).toContain(PREFERRED_WORKFLOW_ID);
    expect(mcp.error.value).toBeNull();
  });

  it('propagates live server errors for unknown tools', async () => {
    const client = await connectClient();

    await expect(client.callTool('DefinitelyMissingTool')).rejects.toThrow(
      'Unknown tool: DefinitelyMissingTool',
    );
    expect(client.error.value).toBe('Unknown tool: DefinitelyMissingTool');
  });

  it('cancels a running pipeline', async () => {
    const mcp = await connectPipelineMcp();
    const workflowId = await pickWorkflowId(mcp);
    const pipelineId = await createPipelineSession(mcp, workflowId, 2);

    const runResult = (await mcp.runPipeline(pipelineId)) as PipelineLifecycleResult;
    expect(runResult).toMatchObject({ status: 'started', sessionId: pipelineId });

    const cancelResult = (await mcp.cancelPipeline(pipelineId)) as PipelineLifecycleResult;
    expect(cancelResult).toMatchObject({ status: 'cancelled', sessionId: pipelineId });

    const cancelledView = await mcp.viewPipeline(pipelineId);
    expect(cancelledView.pipelineId).toBe(pipelineId);
    expect(cancelledView.running).toBe(false);
  });

  it('diffViews reports status transitions without talking to the server', () => {
    const prev = makeView([
      makeViewNode('node-a', 'pending'),
      makeViewNode('node-b', 'active'),
    ]);
    const curr = makeView([
      makeViewNode('node-a', 'active'),
      makeViewNode('node-b', 'completed'),
    ]);

    const { diffViews } = usePipelineMcp();
    const entries = diffViews(prev, curr);

    expect(entries).toHaveLength(2);
    expect(entries[0]).toMatchObject({ nodeId: 'node-a', from: 'pending', to: 'active' });
    expect(entries[1]).toMatchObject({ nodeId: 'node-b', from: 'active', to: 'completed' });
    expect(entries[0].timestamp).toBeGreaterThan(0);
    expect(diffViews(null, curr)).toEqual([]);
    expect(diffViews(prev, null)).toEqual([]);
  });

  it('polls live pipeline views and stops cleanly', async () => {
    const mcp = await connectPipelineMcp();
    const workflowId = await pickWorkflowId(mcp);
    const pipelineId = await createPipelineSession(mcp, workflowId, 1);
    const updates: PipelineView[] = [];

    const stop = mcp.startPolling(
      pipelineId,
      (view) => {
        updates.push(view);
      },
      75,
    );

    await vi.waitFor(
      () => {
        expect(updates.length).toBeGreaterThanOrEqual(2);
      },
      { timeout: 3000, interval: 25 },
    );

    stop();
    const stoppedAt = updates.length;
    await delay(250);

    expect(updates.length).toBe(stoppedAt);
    expect(updates.every((view) => view.pipelineId === pipelineId)).toBe(true);
  });

  it('gracefully reports network failures without connecting', async () => {
    const client = useMcpClient('http://127.0.0.1:1');

    await client.connect();

    expect(client.connected.value).toBe(false);
    expect(client.sessionId.value).toBeNull();
    expect(client.error.value).toEqual(expect.stringMatching(/Network error during initialize|fetch/i));

    await expect(client.disconnect()).resolves.toBeUndefined();
    expect(client.connected.value).toBe(false);
  });

  it('exposes the raw ListAssets envelope shape through callTool before usePipelineMcp normalizes it', async () => {
    const client = await connectClient();
    const result = await client.callTool<unknown>('ListAssets', { type: 'workflow' });
    const workflows = unwrapToolData<Array<{ id: string }>>(result);

    expect(Array.isArray(workflows)).toBe(true);
    expect(workflows.length).toBeGreaterThan(0);
    expect(workflows[0]).toEqual(expect.objectContaining({ id: expect.any(String) }));
  });
});
