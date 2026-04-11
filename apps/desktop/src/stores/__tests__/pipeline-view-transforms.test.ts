import { describe, expect, it } from 'vitest';

import type { PipelineDefinition, PipelineView } from '../../lib/pipeline-types';
import {
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
          model: 'legacy-prompt-model',
          thoughtLevel: 'medium',
          agentConfig: { mode: 'workflow', model: 'fresh-prompt-model' },
        },
        {
          nodeId: 'thinking-1',
          sessionId: 'session-2',
          status: 'pending',
          attempts: 0,
          type: 'sequentialThinking',
          topic: 'Investigate',
          agent: 'architect',
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
          model: 'stale-prompt-model',
          thoughtLevel: 'medium',
          agentConfig: { mode: 'workflow', model: 'fresh-prompt-model' },
        },
        {
          id: 'thinking-1',
          type: 'sequentialThinking',
          topic: 'Investigate',
          agent: 'architect',
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
});
