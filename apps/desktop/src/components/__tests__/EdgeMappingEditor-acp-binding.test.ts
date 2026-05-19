// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import type { VueWrapper } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { Component } from 'vue';

import EdgeMappingEditor from '../EdgeMappingEditor.vue';
import type { PipelineEdge } from '../../lib/pipeline-types-edge';
import type { PipelineNode } from '../../lib/pipeline-types-node';
import { useMcpToolDiscoveryStore } from '../../stores/mcp-tool-discovery';
import { usePipelineStore } from '../../stores/pipeline';

const selectStubs = {
  Select: {
    name: 'Select',
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<div data-testid="select-stub" :data-model-value="modelValue"><slot /></div>',
  },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: {
    props: ['value'],
    template: '<div role="option" :data-value="value"><slot /></div>',
  },
  SelectTrigger: { template: '<button type="button" v-bind="$attrs"><slot /></button>' },
  SelectValue: { props: ['placeholder'], template: '<span>{{ placeholder }}</span>' },
};

const schema = {
  type: 'object',
  properties: {
    topic: { description: 'Topic to investigate' },
    depth: { description: 'How deep to go' },
  },
  required: ['topic'],
};

function edge(overrides: Record<string, unknown> = {}): PipelineEdge {
  return {
    source: 'source',
    target: 'target',
    mode: 'binding',
    ...overrides,
  } as unknown as PipelineEdge;
}

function sourceNode(): PipelineNode {
  return {
    id: 'source',
    type: 'tool',
    toolName: 'maenifold.SearchMemories',
    outputSchema: {
      type: 'object',
      properties: {
        summary: {},
        title: {},
      },
    },
  } as unknown as PipelineNode;
}

function acpTarget(inputSchema?: Record<string, unknown>): PipelineNode {
  return {
    id: 'target',
    type: 'prompt',
    agent: 'architect',
    prompt: 'Summarize the topic',
    ...(inputSchema ? { inputSchema } : {}),
  } as unknown as PipelineNode;
}

function toolTarget(): PipelineNode {
  return {
    id: 'target',
    type: 'tool',
    toolName: 'maenifold.CreateBriefing',
  };
}

function mountEditor(options: {
  edge?: PipelineEdge;
  targetNode?: PipelineNode;
  tools?: Array<{ name: string; description?: string; inputSchema?: unknown }>;
} = {}) {
  const selectedEdge = options.edge ?? edge();
  const targetNode = options.targetNode ?? acpTarget(schema);
  const pinia = createPinia();
  setActivePinia(pinia);
  const pipelineStore = usePipelineStore();
  const discoveryStore = useMcpToolDiscoveryStore();

  pipelineStore.localDefinition = {
    nodes: [sourceNode(), targetNode],
    edges: [selectedEdge],
  };
  pipelineStore.currentView = {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: ['source'],
    mermaid: 'graph TD;',
    edges: [selectedEdge],
    nodes: [
      {
        nodeId: 'source',
        sessionId: 'session-source',
        status: 'completed',
        attempts: 1,
        type: 'tool',
        toolName: 'maenifold.SearchMemories',
        output: JSON.stringify({ summary: 'Found summary', title: 'Found title' }),
      },
    ],
  };
  discoveryStore.serverResults = [
    {
      server: 'maenifold',
      type: 'stdio',
      status: 'ok',
      tools: options.tools ?? [],
    },
  ];

  const wrapper = mount(EdgeMappingEditor as unknown as Component, {
    props: {
      edge: selectedEdge,
      sourceNode: sourceNode(),
      targetNode,
    },
    global: {
      plugins: [pinia],
      stubs: selectStubs,
    },
  });

  return { wrapper, pipelineStore };
}

function textNear(wrapper: VueWrapper, needle: string): string {
  const element = wrapper.get(`[name="${needle}"]`).element;
  return element.closest('label, [data-testid$="-binding-row"], .binding-row')?.textContent ?? '';
}

async function selectMode(wrapper: VueWrapper, mode: 'binding' | 'context' | 'none') {
  const radio = wrapper.find(`input[type="radio"][value="${mode}"]`);
  if (radio.exists()) {
    await radio.setValue(true);
    await flushPromises();
    return;
  }

  const button = wrapper.findAll('button').find((candidate) =>
    candidate.text().trim().toLowerCase() === mode,
  );
  if (button) {
    await button.trigger('click');
    await flushPromises();
    return;
  }

  const select = wrapper.findAllComponents({ name: 'Select' }).find((candidate) =>
    ['context', 'binding', 'none'].every((label) => candidate.text().toLowerCase().includes(label)),
  );
  if (select) {
    select.vm.$emit('update:modelValue', mode);
    await flushPromises();
    return;
  }

  throw new Error(`No mode picker control found for ${mode}`);
}

describe('EdgeMappingEditor ACP binding v2', () => {
  it('renders typed form for a tool target from tool inputSchema', () => {
    const { wrapper } = mountEditor({
      targetNode: toolTarget(),
      tools: [{
        name: 'maenifold.CreateBriefing',
        inputSchema: schema,
      }],
    });

    expect(wrapper.text()).toContain('topic');
    expect(wrapper.text()).toContain('Required');
    expect(wrapper.text()).toContain('Topic to investigate');
    expect(wrapper.text()).toContain('depth');
    expect(wrapper.text()).toContain('Optional');
    expect(wrapper.text()).toContain('How deep to go');
    expect(wrapper.findAll('input[type="text"]')).toHaveLength(2);
  });

  it('renders one labeled ACP binding input per inputSchema property with badge and description', () => {
    const { wrapper } = mountEditor();

    expect(wrapper.get('input[name="topic"]').exists()).toBe(true);
    expect(textNear(wrapper, 'topic')).toContain('Required');
    expect(textNear(wrapper, 'topic')).toContain('Topic to investigate');
    expect(wrapper.get('input[name="depth"]').exists()).toBe(true);
    expect(textNear(wrapper, 'depth')).toContain('Optional');
    expect(textNear(wrapper, 'depth')).toContain('How deep to go');
  });

  it('shows a missing input_schema error chip for binding mode and disables submission', () => {
    const { wrapper } = mountEditor({ targetNode: acpTarget(), edge: edge({ mode: 'binding' }) });

    expect(wrapper.text()).toContain(
      'Target node must declare input_schema before bindings can be configured.',
    );
    const submitControls = wrapper.findAll('button[type="submit"], input[type="submit"]');
    expect(submitControls.length).toBeGreaterThan(0);
    expect(submitControls.every((control) => control.attributes('disabled') !== undefined)).toBe(true);
  });

  it('renders the legacy read-only context notice when edge mode is context', () => {
    const { wrapper } = mountEditor({ edge: edge({ mode: 'context' }) });

    expect(wrapper.text()).toContain(
      'ACP edges receive predecessor context as verbatim text prepend.',
    );
    expect(wrapper.text()).not.toContain('Typed bindings for ACP nodes are not available yet.');
    expect(wrapper.findAll('input[type="text"], textarea')).toHaveLength(0);
  });

  it('renders the read-only control-flow notice when edge mode is none', () => {
    const { wrapper } = mountEditor({ edge: edge({ mode: 'none' }) });

    expect(wrapper.text()).toContain('Control-flow edge — no data crosses.');
    expect(wrapper.findAll('input[type="text"], textarea')).toHaveLength(0);
  });

  it('shows the missing-schema error only when the mode picker selects binding', async () => {
    const { wrapper } = mountEditor({ targetNode: acpTarget(), edge: edge({ mode: 'context' }) });
    const errorText = 'Target node must declare input_schema before bindings can be configured.';

    expect(wrapper.text()).not.toContain(errorText);
    await selectMode(wrapper, 'binding');
    expect(wrapper.text()).toContain(errorText);
    await selectMode(wrapper, 'context');
    expect(wrapper.text()).not.toContain(errorText);
    await selectMode(wrapper, 'none');
    expect(wrapper.text()).not.toContain(errorText);
  });

  it('persists edited ACP binding fields to edge.bindings and removes empty values on blur', async () => {
    const { wrapper, pipelineStore } = mountEditor();

    await wrapper.get('input[name="topic"]').setValue('summary');
    await wrapper.get('input[name="depth"]').setValue('title');
    await flushPromises();
    expect(pipelineStore.localDefinition.edges[0]).toMatchObject({
      bindings: [
        { to: 'topic', from: 'summary' },
        { to: 'depth', from: 'title' },
      ],
    });
    expect(pipelineStore.localDefinition.edges[0]).not.toHaveProperty('mapping');

    await wrapper.get('input[name="depth"]').setValue('');
    await wrapper.get('input[name="depth"]').trigger('blur');
    await flushPromises();
    expect(pipelineStore.localDefinition.edges[0]).toMatchObject({
      bindings: [{ to: 'topic', from: 'summary' }],
    });
  });

  it('marks prompt ACP schema fields as Required and Optional by name', () => {
    const { wrapper } = mountEditor({
      targetNode: acpTarget({
        properties: {
          topic: { description: 'x' },
          depth: {},
        },
        required: ['topic'],
      }),
    });

    expect(textNear(wrapper, 'topic')).toContain('Required');
    expect(textNear(wrapper, 'topic')).toContain('x');
    expect(textNear(wrapper, 'depth')).toContain('Optional');
  });
});
