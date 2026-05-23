import { beforeEach, describe, expect, it, vi } from 'vitest';
import { usePipelineMcp } from '../usePipelineMcp';
import type { PipelineDefinition } from '../../lib/pipeline-types-edge';

const gatewayExecute = vi.fn(async () => ({ data: { pipelineId: 'pipeline-existing' } }));

vi.mock('../useMcpClient', () => ({
  useMcpClient: () => ({
    gatewayExecute,
    connect: vi.fn(),
    connected: { value: true },
    disconnect: vi.fn(),
    error: { value: null },
    listTools: vi.fn(async () => []),
    reconnecting: { value: false },
  }),
}));

describe('usePipelineMcp T-301.26 update dispatch', () => {
  beforeEach(() => {
    gatewayExecute.mockClear();
  });

  it('usePipelineMcp_t301_26_updatePipeline_dispatches_sessionId_definition_to_pipeline_tool', async () => {
    const definition: PipelineDefinition = {
      nodes: [{ id: 'node-a', type: 'tool', toolName: 'example.tool' }],
      edges: [],
    };
    const mcp = usePipelineMcp() as ReturnType<typeof usePipelineMcp> & {
      updatePipeline: (
        sessionId: string,
        definition: PipelineDefinition,
        options?: { name?: string; description?: string },
      ) => Promise<unknown>;
    };

    await mcp.updatePipeline('pipeline-existing', definition, {
      name: 'Renamed',
      description: 'Updated',
    });

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.workflow', 'Pipeline', {
      sessionId: 'pipeline-existing',
      definition,
      name: 'Renamed',
      description: 'Updated',
      update: true,
    });
  });
});
