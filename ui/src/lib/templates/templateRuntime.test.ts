import { describe, expect, it } from 'vitest';
import {
  buildIntentDefinition,
  getAvailableTemplates,
  instantiateTemplate,
  intentPipelineName,
  interpolateValue,
  isTemplateAvailable,
  missingRequiredInputs,
  resolveAgents,
  slugifyPipelineName,
  templateAvailability,
} from './templateRuntime';
import { PIPELINE_TEMPLATES } from './pipelineTemplates';
import type { PipelineTemplate, TemplateVariables } from './pipelineTemplates';

const universalTemplate: PipelineTemplate = {
  id: 'demo-universal',
  label: 'Demo',
  description: 'demo',
  icon: 'flask-conical',
  category: 'universal',
  capabilities: [],
  definition: {
    nodes: [
      { id: 'a', workflowId: 'wf', acpClient: '{{primaryAgent}}', prompt: '{{inputs.topic}}' },
      { type: 'prompt', id: 'b', prompt: 'review', agent: '{{criticAgent}}' },
    ],
    edges: [{ source: 'a', target: 'b', type: 'success' }],
  },
  runInputs: [{ key: 'topic', label: 'Topic', required: true }],
  nodeLabels: { a: 'First', b: 'Second' },
};

const enterpriseTemplate: PipelineTemplate = {
  id: 'demo-enterprise',
  label: 'Enterprise',
  description: 'gated',
  icon: 'briefcase',
  category: 'enterprise',
  capabilities: [{ toolName: 'tool-one' }, { toolName: 'tool-two' }],
  definition: {
    nodes: [
      { type: 'tool', id: 't', toolName: 'tool-one', toolArgs: { account: '{{inputs.accountName}}' } },
    ],
    edges: [],
  },
  runInputs: [{ key: 'accountName', label: 'Account', required: true }],
  nodeLabels: { t: 'Tool' },
};

describe('resolveAgents', () => {
  it('returns empty agents when none configured', () => {
    expect(resolveAgents([])).toEqual({ primaryAgent: '', criticAgent: '' });
  });

  it('self-critiques with a single agent', () => {
    expect(resolveAgents(['solo'])).toEqual({ primaryAgent: 'solo', criticAgent: 'solo' });
  });

  it('uses distinct primary and critic with two agents', () => {
    expect(resolveAgents(['one', 'two'])).toEqual({ primaryAgent: 'one', criticAgent: 'two' });
  });

  it('honours a valid primary preference', () => {
    expect(resolveAgents(['one', 'two', 'three'], 'two')).toEqual({
      primaryAgent: 'two',
      criticAgent: 'one',
    });
  });

  it('falls back to the first agent for an unknown preference', () => {
    expect(resolveAgents(['one', 'two'], 'ghost')).toEqual({
      primaryAgent: 'one',
      criticAgent: 'two',
    });
  });
});

describe('interpolateValue', () => {
  const variables: TemplateVariables = {
    primaryAgent: 'P',
    criticAgent: 'C',
    inputs: { topic: 'climate', accountName: 'Contoso' },
  };

  it('interpolates a string leaf', () => {
    expect(interpolateValue('{{primaryAgent}} reviews {{inputs.topic}}', variables)).toBe(
      'P reviews climate'
    );
  });

  it('recurses into nested objects and arrays (toolArgs preserved)', () => {
    const value = {
      toolArgs: { account: '{{inputs.accountName}}', nested: { who: '{{criticAgent}}' } },
      list: ['{{inputs.topic}}', 'static'],
      count: 8,
    };
    expect(interpolateValue(value, variables)).toEqual({
      toolArgs: { account: 'Contoso', nested: { who: 'C' } },
      list: ['climate', 'static'],
      count: 8,
    });
  });

  it('leaves unknown placeholders untouched', () => {
    expect(interpolateValue('{{inputs.missing}}', variables)).toBe('{{inputs.missing}}');
  });
});

describe('instantiateTemplate', () => {
  const variables: TemplateVariables = {
    primaryAgent: 'Alice',
    criticAgent: 'Bob',
    inputs: { topic: 'oceans' },
  };

  it('assigns fresh ids, stamps displayLabels, and interpolates', () => {
    const { definition, idMap } = instantiateTemplate(universalTemplate, variables);

    expect(definition.nodes).toHaveLength(2);
    const [first, second] = definition.nodes;

    // Fresh ids differ from the authoring ids.
    expect(first.id).not.toBe('a');
    expect(second.id).not.toBe('b');
    expect(idMap.get('a')).toBe(first.id);
    expect(idMap.get('b')).toBe(second.id);

    // displayLabel stamped from nodeLabels.
    expect(first.displayLabel).toBe('First');
    expect(second.displayLabel).toBe('Second');

    // Interpolation applied.
    expect((first as { acpClient?: string }).acpClient).toBe('Alice');
    expect((first as { prompt?: string }).prompt).toBe('oceans');
    expect((second as { agent?: string }).agent).toBe('Bob');
  });

  it('remaps edge endpoints to the fresh ids', () => {
    const { definition, idMap } = instantiateTemplate(universalTemplate, variables);
    const [edge] = definition.edges;
    expect(edge.source).toBe(idMap.get('a'));
    expect(edge.target).toBe(idMap.get('b'));
    expect(edge.type).toBe('success');
  });

  it('does not mutate the source template', () => {
    instantiateTemplate(universalTemplate, variables);
    expect(universalTemplate.definition.nodes[0].id).toBe('a');
    expect((universalTemplate.definition.nodes[0] as { acpClient?: string }).acpClient).toBe(
      '{{primaryAgent}}'
    );
  });

  it('preserves nested toolArgs through interpolation', () => {
    const { definition } = instantiateTemplate(enterpriseTemplate, {
      primaryAgent: 'P',
      criticAgent: 'C',
      inputs: { accountName: 'Globex' },
    });
    const toolNode = definition.nodes[0] as { toolArgs?: Record<string, unknown> };
    expect(toolNode.toolArgs).toEqual({ account: 'Globex' });
  });
});

describe('capability gating', () => {
  it('universal templates are always available regardless of status', () => {
    expect(templateAvailability(universalTemplate, new Set(), 'idle')).toBe('available');
    expect(templateAvailability(universalTemplate, new Set(), 'loading')).toBe('available');
  });

  it('enterprise template fails closed while discovery is not ready', () => {
    const allTools = new Set(['tool-one', 'tool-two']);
    expect(templateAvailability(enterpriseTemplate, allTools, 'loading')).toBe('discovery-pending');
    expect(templateAvailability(enterpriseTemplate, allTools, 'error')).toBe('discovery-pending');
    expect(templateAvailability(enterpriseTemplate, allTools, 'idle')).toBe('discovery-pending');
  });

  it('enterprise template becomes available only when ALL tools present and ready', () => {
    expect(templateAvailability(enterpriseTemplate, new Set(['tool-one']), 'ready')).toBe(
      'unavailable'
    );
    expect(
      templateAvailability(enterpriseTemplate, new Set(['tool-one', 'tool-two']), 'ready')
    ).toBe('available');
  });

  it('isTemplateAvailable requires all gates', () => {
    expect(isTemplateAvailable(enterpriseTemplate, new Set(['tool-one']))).toBe(false);
    expect(isTemplateAvailable(enterpriseTemplate, new Set(['tool-one', 'tool-two']))).toBe(true);
    expect(isTemplateAvailable(universalTemplate, new Set())).toBe(true);
  });

  it('getAvailableTemplates hides gated templates until ready', () => {
    const templates = [universalTemplate, enterpriseTemplate];
    expect(getAvailableTemplates(templates, new Set(), 'loading')).toEqual([universalTemplate]);
    expect(
      getAvailableTemplates(templates, new Set(['tool-one', 'tool-two']), 'ready')
    ).toEqual(templates);
  });
});

describe('intent track', () => {
  it('builds a single prompt node carrying the intent and agent', () => {
    const definition = buildIntentDefinition('Investigate X', 'Alice');
    expect(definition.nodes).toHaveLength(1);
    expect(definition.edges).toHaveLength(0);
    const node = definition.nodes[0];
    expect(node.type).toBe('prompt');
    expect((node as { prompt?: string }).prompt).toBe('Investigate X');
    expect((node as { agent?: string }).agent).toBe('Alice');
    expect(node.displayLabel).toBe('Intent');
    expect(node.id.length).toBeGreaterThan(0);
  });

  it('derives a clipped pipeline name', () => {
    expect(intentPipelineName('  Hello world\nsecond line')).toBe('Hello world');
    expect(intentPipelineName('')).toBe('Intent pipeline');
    const long = 'x'.repeat(80);
    expect(intentPipelineName(long).length).toBeLessThanOrEqual(60);
    expect(intentPipelineName(long).endsWith('…')).toBe(true);
  });
});

describe('missingRequiredInputs', () => {
  it('flags missing required inputs', () => {
    expect(missingRequiredInputs(universalTemplate, {})).toEqual(['topic']);
    expect(missingRequiredInputs(universalTemplate, { topic: '   ' })).toEqual(['topic']);
    expect(missingRequiredInputs(universalTemplate, { topic: 'oceans' })).toEqual([]);
  });
});

describe('slugifyPipelineName', () => {
  const SLUG_RE = /^[A-Za-z][A-Za-z0-9-]*$/;

  it('collapses non-alphanumeric runs into single dashes', () => {
    const slug = slugifyPipelineName('Research & Verify', 'fallback');
    expect(slug).toBe('Research-Verify');
    expect(slug).toMatch(SLUG_RE);
  });

  it('strips leading non-letters so the slug starts with a letter', () => {
    expect(slugifyPipelineName('123 Topic', 'fallback')).toBe('Topic');
    expect(slugifyPipelineName('   spaced', 'fallback')).toBe('spaced');
    expect(slugifyPipelineName('---abc', 'fallback')).toBe('abc');
  });

  it('strips trailing dashes', () => {
    expect(slugifyPipelineName('hello world!!!', 'fallback')).toBe('hello-world');
  });

  it('returns the fallback when nothing valid remains', () => {
    expect(slugifyPipelineName('', 'pipeline')).toBe('pipeline');
    expect(slugifyPipelineName('   ', 'pipeline')).toBe('pipeline');
    expect(slugifyPipelineName('!@#$%', 'pipeline')).toBe('pipeline');
    expect(slugifyPipelineName('123456', 'pipeline')).toBe('pipeline');
  });

  it('clips to 64 characters without leaving a trailing dash', () => {
    const slug = slugifyPipelineName(`a${' x'.repeat(80)}`, 'fallback');
    expect(slug.length).toBeLessThanOrEqual(64);
    expect(slug).toMatch(SLUG_RE);
    expect(slug.endsWith('-')).toBe(false);
  });

  it('produces a backend-valid slug from the intent name', () => {
    const slug = slugifyPipelineName(intentPipelineName('Investigate ocean currents'), 'pipeline');
    expect(slug).toMatch(SLUG_RE);
  });
});

describe('catalog node type discriminator (defect B1 guard)', () => {
  const variables: TemplateVariables = {
    primaryAgent: 'P',
    criticAgent: 'C',
    inputs: { topic: 'T', accountName: 'Acme' },
  };

  it('every instantiated node carries a non-empty type for every template', () => {
    for (const template of PIPELINE_TEMPLATES) {
      const { definition } = instantiateTemplate(template, variables);
      expect(definition.nodes.length).toBeGreaterThan(0);
      for (const node of definition.nodes) {
        expect(typeof node.type).toBe('string');
        expect((node.type ?? '').length).toBeGreaterThan(0);
      }
    }
  });

  it('produces backend-valid slugs from every template label', () => {
    const SLUG_RE = /^[A-Za-z][A-Za-z0-9-]*$/;
    for (const template of PIPELINE_TEMPLATES) {
      const slug = slugifyPipelineName(template.label, template.id);
      expect(slug).toMatch(SLUG_RE);
      expect(slug.length).toBeLessThanOrEqual(64);
    }
  });
});
