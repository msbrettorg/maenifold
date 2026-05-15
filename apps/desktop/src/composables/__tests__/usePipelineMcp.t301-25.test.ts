// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PipelineCreateOptions } from '../usePipelineMcp';
import { usePipelineMcp } from '../usePipelineMcp';
import type { PipelineDefinition } from '../../lib/pipeline-types';

const mcpState = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    callTool: vi.fn(async () => ({ success: true, data: { pipelineId: 'created' } })),
    connected: vue.ref(true),
    error: vue.ref<string | null>(null),
    reconnecting: vue.ref(false),
  };
});

vi.mock('../useMcpClient', () => ({
  useMcpClient: () => ({
    callTool: mcpState.callTool,
    connected: mcpState.connected,
    error: mcpState.error,
    reconnecting: mcpState.reconnecting,
  }),
}));

const definition: PipelineDefinition = {
  nodes: [{ id: 'tool', type: 'tool', toolName: 'GetConfig' }],
  edges: [],
};

describe('usePipelineMcp createPipeline T-301.25 metadata payload', () => {
  beforeEach(() => {
    mcpState.callTool.mockClear();
  });

  it('createPipeline_t301_25_includes_name_and_description_in_payload', async () => {
    const options = {
      name: ' user-pipeline ',
      description: ' test desc ',
    } satisfies PipelineCreateOptions;
    const mcp = usePipelineMcp('http://127.0.0.1:8080');

    await mcp.createPipeline(definition, 3, options);

    expect(mcpState.callTool).toHaveBeenCalledWith('Pipeline', {
      definition,
      maxConcurrency: 3,
      name: 'user-pipeline',
      description: 'test desc',
    });
  });

  it('createPipeline_t301_25_omits_blank_description_from_payload', async () => {
    const options = {
      name: 'user-pipeline',
      description: '   ',
    } satisfies PipelineCreateOptions;
    const mcp = usePipelineMcp('http://127.0.0.1:8080');

    await mcp.createPipeline(definition, 2, options);

    expect(mcpState.callTool).toHaveBeenCalledWith('Pipeline', {
      definition,
      maxConcurrency: 2,
      name: 'user-pipeline',
    });
  });
});
