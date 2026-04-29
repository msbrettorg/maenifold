import { describe, expect, it } from 'vitest';

import type { PipelineDefinition, PipelineView } from '../../lib/pipeline-types';
import {
  normalizeView,
  serializePipelineDefinition,
  toLocalDefinition,
} from '../pipeline-view-transforms';

describe('pipeline view transforms', () => {
  it('normalizes viewed execution fields into nested agentConfig for editable nodes', () => {
    const view: PipelineView = {
      pipelineId: 'pipeline-1',
      status: 'paused',
      running: false,
      ready: [],
      active: [],
      completed: [],
      mermaid: 'graph TD;',
      edges: [],
      nodes: [
        {
          nodeId: 'prompt-1',
          sessionId: 'session-1',
          status: 'pending',
          attempts: 0,
          type: 'prompt',
          prompt: 'Investigate',
          agent: 'architect',
          adopt: { role: 'architect' },
          requiresRole: 'architect',
          model: 'legacy-prompt-model',
          thoughtLevel: 'medium',
          agentConfig: { mode: 'workflow', model: 'fresh-prompt-model' },
          sessionRef: { agent: 'architect', sessionId: 'session-ref-1' },
        },
        {
          nodeId: 'thinking-1',
          sessionId: 'session-2',
          status: 'pending',
          attempts: 0,
          type: 'sequentialThinking',
          topic: 'Investigate',
          agent: 'architect',
          adopt: { color: 'blue' },
          requiresRole: 'reviewer',
          model: 'legacy-thinking-model',
          thoughtLevel: 'low',
          agentConfig: { mode: 'prompt', thoughtLevel: 'high' },
        },
        {
          nodeId: 'tool-1',
          sessionId: 'session-3',
          status: 'pending',
          attempts: 0,
          type: 'tool',
          toolName: 'filesystem.ReadFile',
          agent: 'architect',
          model: 'legacy-tool-model',
          thoughtLevel: 'high',
        },
      ],
    };

    const definition = toLocalDefinition(view);
    const [promptNode, thinkingNode, toolNode] = definition.nodes;

    expect(promptNode).toMatchObject({
      id: 'prompt-1',
      type: 'prompt',
      agentConfig: {
        mode: 'workflow',
        model: 'fresh-prompt-model',
        thoughtLevel: 'medium',
      },
      adopt: { role: 'architect' },
      requiresRole: 'architect',
      sessionRef: { agent: 'architect', sessionId: 'session-ref-1' },
    });
    expect('model' in promptNode!).toBe(false);
    expect('thoughtLevel' in promptNode!).toBe(false);

    expect(thinkingNode).toMatchObject({
      id: 'thinking-1',
      type: 'sequentialThinking',
      agentConfig: {
        mode: 'prompt',
        model: 'legacy-thinking-model',
        thoughtLevel: 'high',
      },
      adopt: { color: 'blue' },
      requiresRole: 'reviewer',
    });
    expect('model' in thinkingNode!).toBe(false);
    expect('thoughtLevel' in thinkingNode!).toBe(false);

    expect(toolNode).toMatchObject({
      id: 'tool-1',
      type: 'tool',
      agentConfig: {
        model: 'legacy-tool-model',
        thoughtLevel: 'high',
      },
    });
    expect('model' in toolNode!).toBe(false);
    expect('thoughtLevel' in toolNode!).toBe(false);
  });

  it('strips legacy top-level execution fields before pipeline submission', () => {
    const definition: PipelineDefinition = {
      edges: [],
      nodes: [
        {
          id: 'prompt-1',
          type: 'prompt',
          prompt: 'Investigate',
          agent: 'architect',
          sessionRef: { agent: 'architect', sessionId: 'session-ref-2' },
          adopt: { perspective: 'critical-analyst' },
          requiresRole: 'architect',
          model: 'stale-prompt-model',
          thoughtLevel: 'medium',
          agentConfig: { mode: 'workflow', model: 'fresh-prompt-model' },
        },
        {
          id: 'thinking-1',
          type: 'sequentialThinking',
          topic: 'Investigate',
          agent: 'architect',
          adopt: { role: 'architect', color: 'blue' },
          requiresRole: 'reviewer',
          model: 'stale-thinking-model',
          thoughtLevel: 'low',
          agentConfig: { mode: 'prompt', thoughtLevel: 'high' },
        },
        {
          id: 'tool-1',
          type: 'tool',
          toolName: 'filesystem.ReadFile',
          agent: 'architect',
          model: 'stale-tool-model',
          thoughtLevel: 'low',
          agentConfig: { model: 'fresh-tool-model' },
        },
      ],
    };

    const serialized = serializePipelineDefinition(definition);
    const [promptNode, thinkingNode, toolNode] = serialized.nodes;

    expect(promptNode).toMatchObject({
      id: 'prompt-1',
      type: 'prompt',
      agentConfig: {
        mode: 'workflow',
        model: 'fresh-prompt-model',
        thoughtLevel: 'medium',
      },
      sessionRef: { agent: 'architect', sessionId: 'session-ref-2' },
      adopt: { perspective: 'critical-analyst' },
      requiresRole: 'architect',
    });
    expect('model' in promptNode!).toBe(false);
    expect('thoughtLevel' in promptNode!).toBe(false);

    expect(thinkingNode).toMatchObject({
      id: 'thinking-1',
      type: 'sequentialThinking',
      agentConfig: {
        mode: 'prompt',
        model: 'stale-thinking-model',
        thoughtLevel: 'high',
      },
      adopt: { role: 'architect', color: 'blue' },
      requiresRole: 'reviewer',
    });
    expect('model' in thinkingNode!).toBe(false);
    expect('thoughtLevel' in thinkingNode!).toBe(false);

    expect(toolNode).toMatchObject({
      id: 'tool-1',
      type: 'tool',
      agentConfig: {
        model: 'fresh-tool-model',
        thoughtLevel: 'low',
      },
    });
    expect('model' in toolNode!).toBe(false);
    expect('thoughtLevel' in toolNode!).toBe(false);
  });

  it('preserves ACP session and tool provenance view fields', () => {
    const original: PipelineView = {
      pipelineId: 'pipeline-2',
      status: 'active',
      running: true,
      ready: [],
      active: ['prompt-1'],
      completed: ['tool-1'],
      mermaid: 'graph TD;',
      edges: [],
      nodes: [
        {
          nodeId: 'prompt-1',
          sessionId: 'local-prompt-1',
          acpSessionId: 'acp-remote-1',
          provenance: {
            agent: 'architect',
            inspectable: true,
            detail: {
              sessionId: 'acp-remote-1',
            },
          },
          status: 'active',
          attempts: 1,
          type: 'prompt',
          prompt: 'Reconnect me',
          agent: 'architect',
        },
        {
          nodeId: 'tool-1',
          sessionId: 'local-tool-1',
          provenance: {
            agent: null,
            inspectable: true,
            detail: {},
          },
          status: 'completed',
          attempts: 1,
          type: 'tool',
          toolName: 'example.echo',
          toolProvenance: {
            toolName: 'example.echo',
            resolvedArgs: { value: 1 },
            responseSummary: '{"value":1}',
          },
        },
      ],
    };

    const normalized = normalizeView(original);
    expect(normalized.nodes[0]).toMatchObject({
      nodeId: 'prompt-1',
      acpSessionId: 'acp-remote-1',
      provenance: {
        agent: 'architect',
        inspectable: true,
        detail: {
          sessionId: 'acp-remote-1',
        },
      },
    });
    expect(normalized.nodes[1]).toMatchObject({
      nodeId: 'tool-1',
      provenance: {
        agent: null,
        inspectable: true,
        detail: {},
      },
      toolProvenance: {
        toolName: 'example.echo',
        resolvedArgs: { value: 1 },
        responseSummary: '{"value":1}',
      },
    });

    if (normalized.nodes[1]?.type === 'tool' && normalized.nodes[1].toolProvenance?.resolvedArgs) {
      normalized.nodes[1].toolProvenance.resolvedArgs.value = 2;
    }
    if (normalized.nodes[0]?.provenance?.detail) {
      normalized.nodes[0].provenance.detail.sessionId = 'acp-mutated';
    }

    expect(
      original.nodes[1]?.type === 'tool' ? original.nodes[1].toolProvenance?.resolvedArgs?.value : undefined,
    ).toBe(1);
    expect(original.nodes[0]?.provenance?.detail.sessionId).toBe('acp-remote-1');
  });
});
