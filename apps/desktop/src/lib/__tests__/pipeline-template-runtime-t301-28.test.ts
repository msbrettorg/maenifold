import { describe, expect, it } from 'vitest';
import { instantiateTemplate } from '../pipeline-template-runtime';
import type { PipelineTemplate } from '../pipeline-templates';

function fixedPromptTemplate(): PipelineTemplate {
  return {
    id: 't301-28-fixed-prompt-template',
    label: 'T301 Fixed Prompt Template',
    description: 'Regression fixture for same-template pipeline joins.',
    icon: 'message-square',
    category: 'universal',
    capabilities: [],
    definition: {
      nodes: [
        {
          id: 'prompt-fixed-uuid-1',
          type: 'prompt',
          prompt: 'Draft {{inputs.topic}}',
          agent: '{{primaryAgent}}',
        },
        {
          id: 'prompt-fixed-uuid-2',
          type: 'prompt',
          prompt: 'Review {{inputs.topic}}',
          agent: '{{criticAgent}}',
        },
      ],
      edges: [
        {
          source: 'prompt-fixed-uuid-1',
          target: 'prompt-fixed-uuid-2',
          type: 'success',
        },
      ],
    },
    runInputs: [],
    nodeLabels: {
      'prompt-fixed-uuid-1': 'Draft',
      'prompt-fixed-uuid-2': 'Review',
    },
  };
}

const vars = {
  primaryAgent: 'primary-agent',
  criticAgent: 'critic-agent',
  inputs: { topic: 'same-template join' },
};

describe('instantiateTemplate T-301.28 node identity', () => {
  it('instantiateTemplate_t301_28_generates_fresh_node_ids_per_invocation', () => {
    const template = fixedPromptTemplate();
    const first = instantiateTemplate(template, vars);
    const second = instantiateTemplate(template, vars);

    const firstIds = first.definition.nodes.map((node) => node.id);
    const secondIds = second.definition.nodes.map((node) => node.id);

    expect(firstIds).not.toContain('prompt-fixed-uuid-1');
    expect(firstIds).not.toContain('prompt-fixed-uuid-2');
    expect(secondIds).not.toContain('prompt-fixed-uuid-1');
    expect(secondIds).not.toContain('prompt-fixed-uuid-2');
    expect(new Set([...firstIds, ...secondIds]).size).toBe(4);
    expect(firstIds.every((id) => id.startsWith('prompt-'))).toBe(true);
    expect(secondIds.every((id) => id.startsWith('prompt-'))).toBe(true);
  });

  it('instantiateTemplate_t301_28_remaps_edges_to_new_node_ids', () => {
    const result = instantiateTemplate(fixedPromptTemplate(), vars);
    const firstNodeId = result.definition.nodes[0].id;
    const secondNodeId = result.definition.nodes[1].id;
    const edge = result.definition.edges[0];

    expect(firstNodeId).not.toBe('prompt-fixed-uuid-1');
    expect(secondNodeId).not.toBe('prompt-fixed-uuid-2');
    expect(edge.source).toBe(firstNodeId);
    expect(edge.target).toBe(secondNodeId);
    expect(edge.source).not.toBe('prompt-fixed-uuid-1');
    expect(edge.target).not.toBe('prompt-fixed-uuid-2');
  });

  it('instantiateTemplate_t301_28_preserves_node_count_and_edge_count', () => {
    const template = fixedPromptTemplate();
    const result = instantiateTemplate(template, vars);

    expect(result.definition.nodes).toHaveLength(template.definition.nodes.length);
    expect(result.definition.edges).toHaveLength(template.definition.edges.length);
    expect(result.definition.nodes.map((node) => node.type)).toEqual(['prompt', 'prompt']);
    expect(result.definition.nodes.map((node) => node.id)).not.toEqual([
      'prompt-fixed-uuid-1',
      'prompt-fixed-uuid-2',
    ]);
  });

  it('instantiateTemplate_t301_28_handles_template_without_edges', () => {
    const template = fixedPromptTemplate();
    template.definition.edges = [];

    const result = instantiateTemplate(template, vars);

    expect(result.definition.nodes).toHaveLength(2);
    expect(result.definition.edges).toEqual([]);
    expect(result.definition.nodes.map((node) => node.id)).not.toEqual([
      'prompt-fixed-uuid-1',
      'prompt-fixed-uuid-2',
    ]);
  });
});
