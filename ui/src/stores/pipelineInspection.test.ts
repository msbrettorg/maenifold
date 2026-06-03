import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';

// Blue-team coverage (T-600.7): the inspection store owns two safety behaviors —
// grace/hidden-inference tracking for ALL active nodes, and the single live ACP
// attachment. We mock the ACP bridge (no real transport) and drive the store
// through attach → stream → detach plus the grace derivation.

const bridges: FakeBridge[] = [];

class FakeBridge {
  onSessionUpdate: ((n: unknown) => void) | null = null;
  onTransportClose: ((reason?: string) => void) | null = null;
  initialize = vi.fn().mockResolvedValue(undefined);
  loadSession = vi.fn().mockResolvedValue(undefined);
  disconnect = vi.fn().mockResolvedValue(undefined);
}

const createAcpClientMock = vi.fn(async (..._args: unknown[]) => {
  const b = new FakeBridge();
  bridges.push(b);
  return b;
});

vi.mock('../lib/acp-bridge', () => ({
  AcpClientBridge: class {},
  createAcpClient: (...args: unknown[]) => createAcpClientMock(...args),
}));

vi.mock('../lib/mcp/gateway', () => ({
  executeGateway: vi.fn(),
  readGatewayResource: vi.fn(),
}));

import { usePipelineInspectionStore } from './pipelineInspection';
import { useConfigStore } from './config';
import { usePipelinesStore } from './pipelines';
import type { AgentsConfig } from '../lib/types';
import type { PipelineView, PromptViewNode, ToolViewNode, ViewNode } from '../lib/pipeline-types';

function promptNode(over: Partial<PromptViewNode> = {}): PromptViewNode {
  return {
    type: 'prompt',
    nodeId: 'n-prompt',
    sessionId: 'pipeline-sess',
    status: 'active',
    attempts: 1,
    prompt: 'go',
    agent: 'copilot',
    model: 'gpt-5',
    acpSessionId: 'acp-1',
    ...over,
  };
}

function viewWith(nodes: ViewNode[], active: string[]): PipelineView {
  return {
    pipelineId: 'p1',
    name: 'p1',
    status: 'active',
    nodes,
    edges: [],
    ready: [],
    active,
    completed: [],
    mermaid: '',
    running: true,
  };
}

function setAgents(): void {
  const config = useConfigStore();
  const agents: AgentsConfig = {
    agents: { copilot: { url: 'ws://localhost/x' } as never },
  };
  config.updateFromEvent(agents);
}

beforeEach(() => {
  setActivePinia(createPinia());
  bridges.length = 0;
  createAcpClientMock.mockClear();
});

describe('syncActiveNodes — grace tracking', () => {
  it('records a grace start for each active inspectable node and drops inactive ones', () => {
    const store = usePipelineInspectionStore();
    const node = promptNode();
    const tool = { ...({} as ToolViewNode), type: 'tool', nodeId: 'n-tool', sessionId: 's', status: 'active', attempts: 1, toolName: 't' } as ToolViewNode;
    store.syncActiveNodes(viewWith([node, tool], ['n-prompt', 'n-tool']));
    // tool nodes are not tracked (not inspectable)
    expect(store.graceStartByNode.has('n-prompt')).toBe(true);
    expect(store.graceStartByNode.has('n-tool')).toBe(false);

    // once the node is no longer active its grace entry is removed
    store.syncActiveNodes(viewWith([node], []));
    expect(store.graceStartByNode.has('n-prompt')).toBe(false);
  });
});

describe('hiddenErrors derivation', () => {
  it('lists active nodes that fail the guard past grace', () => {
    const store = usePipelineInspectionStore();
    const pipelines = usePipelinesStore();
    const blind = promptNode({ nodeId: 'n-blind', acpSessionId: null, model: null });
    pipelines.selectedView = viewWith([blind], ['n-blind']);
    store.graceStartByNode = new Map([['n-blind', Date.now() - 40_000]]);
    store.nowTick = Date.now();
    expect(store.hiddenErrors).toHaveLength(1);
    expect(store.hiddenErrors[0]).toMatchObject({ nodeId: 'n-blind', missing: ['sessionId', 'model'] });
  });

  it('is empty when the active node is fully inspectable', () => {
    const store = usePipelineInspectionStore();
    const pipelines = usePipelinesStore();
    pipelines.selectedView = viewWith([promptNode()], ['n-prompt']);
    store.graceStartByNode = new Map([['n-prompt', Date.now() - 40_000]]);
    store.nowTick = Date.now();
    expect(store.hiddenErrors).toEqual([]);
  });
});

describe('attach / live stream / detach', () => {
  it('connects to the node session and folds streamed updates into messages', async () => {
    setAgents();
    const store = usePipelineInspectionStore();
    await store.attach(promptNode());

    expect(createAcpClientMock).toHaveBeenCalledTimes(1);
    // Defect-1 guard: the observer MUST force the bridge's fs handlers off. The
    // bridge resolves `options?.fsAvailable ?? hasLocalFs()` (acp-bridge.ts:79),
    // so passing fsAvailable:false makes the read-only guarantee independent of
    // hasLocalFs() — fs/read_text_file & fs/write_text_file answer not-found.
    expect(createAcpClientMock).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'copilot' }),
      { fsAvailable: false }
    );
    expect(store.attachState).toBe('live');
    expect(store.attachedNodeId).toBe('n-prompt');
    const bridge = bridges[0];
    expect(bridge.initialize).toHaveBeenCalledTimes(1);
    expect(bridge.loadSession).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'acp-1', mcpServers: [] })
    );

    // simulate the daemon replaying a message
    bridge.onSessionUpdate?.({
      sessionId: 'acp-1',
      update: { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: 'hi' } },
    });
    await nextTick();
    expect(store.messages).toHaveLength(1);
    expect(store.messages[0]).toMatchObject({ role: 'assistant', content: 'hi' });

    await store.detach();
    expect(bridge.disconnect).toHaveBeenCalledTimes(1);
    expect(store.attachState).toBe('idle');
    expect(store.attachedNodeId).toBeNull();
  });

  it('does not connect for a node that fails the guard (blocked, no transport)', async () => {
    setAgents();
    const store = usePipelineInspectionStore();
    await store.attach(promptNode({ acpSessionId: null, sessionRef: undefined }));
    expect(createAcpClientMock).not.toHaveBeenCalled();
    expect(store.attachState).toBe('blocked');
  });

  it('is a no-op for tool nodes', async () => {
    setAgents();
    const store = usePipelineInspectionStore();
    const tool = { type: 'tool', nodeId: 'n-tool', sessionId: 's', status: 'active', attempts: 1, toolName: 't' } as ToolViewNode;
    await store.attach(tool);
    expect(createAcpClientMock).not.toHaveBeenCalled();
    expect(store.attachedNodeId).toBeNull();
  });

  it('errors when the agent is missing from config', async () => {
    // no setAgents() → getAgent returns undefined
    const store = usePipelineInspectionStore();
    await store.attach(promptNode());
    expect(store.attachState).toBe('error');
    expect(store.attachError).toContain('copilot');
  });
});

describe('attachment identity (Defect 2: no stale stream bleed)', () => {
  it('reconnects with a fresh buffer when the pipeline changes under a reused node id', async () => {
    setAgents();
    const store = usePipelineInspectionStore();
    const pipelines = usePipelinesStore();

    pipelines.selectedPipelineId = 'pA';
    await store.attach(promptNode());
    expect(store.attachState).toBe('live');
    const firstBridge = bridges[0];

    // stream a message into pipeline A's buffer
    firstBridge.onSessionUpdate?.({
      sessionId: 'acp-1',
      update: { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: 'from-A' } },
    });
    await nextTick();
    expect(store.messages).toHaveLength(1);

    // SAME node id + session, DIFFERENT pipeline → distinct identity, must
    // tear down and reconnect; the previous stream must NOT bleed through.
    pipelines.selectedPipelineId = 'pB';
    await store.attach(promptNode());
    expect(createAcpClientMock).toHaveBeenCalledTimes(2);
    expect(firstBridge.disconnect).toHaveBeenCalledTimes(1);
    expect(store.messages).toHaveLength(0);
  });

  it('reconnects when the resolved session is upgraded (provenance.detail wins)', async () => {
    setAgents();
    const store = usePipelineInspectionStore();
    await store.attach(promptNode());
    expect(store.attachState).toBe('live');
    const firstBridge = bridges[0];
    expect(firstBridge.loadSession).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'acp-1' })
    );

    // Same node id, but the session is upgraded: provenance.detail.sessionId has
    // priority over acpSessionId, so the resolved session changes acp-1 → acp-2.
    await store.attach(
      promptNode({ provenance: { agent: 'copilot', inspectable: true, detail: { sessionId: 'acp-2' } } })
    );
    expect(createAcpClientMock).toHaveBeenCalledTimes(2);
    expect(firstBridge.disconnect).toHaveBeenCalledTimes(1);
    expect(bridges[1].loadSession).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'acp-2' })
    );
  });

  it('is idempotent (no reconnect) for an unchanged identity', async () => {
    setAgents();
    const store = usePipelineInspectionStore();
    const pipelines = usePipelinesStore();
    pipelines.selectedPipelineId = 'pA';
    await store.attach(promptNode());
    expect(createAcpClientMock).toHaveBeenCalledTimes(1);
    // identical pipeline + node + session + agent + model → no-op
    await store.attach(promptNode());
    expect(createAcpClientMock).toHaveBeenCalledTimes(1);
    expect(bridges[0].disconnect).not.toHaveBeenCalled();
  });
});
