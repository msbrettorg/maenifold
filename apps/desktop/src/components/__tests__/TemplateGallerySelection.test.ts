// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import TemplateGallerySelection from '../TemplateGallerySelection.vue';
import { instantiateTemplate } from '../../lib/pipeline-template-runtime';
import type { PipelineTemplate } from '../../lib/pipeline-templates';

const baseTemplate: PipelineTemplate = {
  id: 'research-and-verify',
  label: 'Research & Verify',
  description: 'Investigate a topic and review it.',
  icon: 'flask-conical',
  category: 'universal',
  capabilities: [],
  definition: {
    nodes: [
      { id: 'investigate', type: 'workflow', workflowId: 'scientific-method', acpClient: '{{primaryAgent}}', prompt: '{{inputs.topic}}' },
    ],
    edges: [],
  },
  runInputs: [
    { key: 'topic', label: 'Topic or question', placeholder: 'Research X', required: true },
    { key: 'notes', label: 'Notes', type: 'textarea', required: false },
  ],
  nodeLabels: { investigate: 'Investigate Topic' },
};

function mountSelection(template: PipelineTemplate = baseTemplate) {
  return mount(TemplateGallerySelection, {
    props: { canCreate: true, hasIntentAgent: true, template },
  });
}

describe('TemplateGallerySelection', () => {
  it('renders editable fields and blocks create while a required input is empty', () => {
    const wrapper = mountSelection();

    const topic = wrapper.find('input#template-input-research-and-verify-topic');
    const notes = wrapper.find('textarea#template-input-research-and-verify-notes');

    expect(topic.exists()).toBe(true);
    expect(notes.exists()).toBe(true);
    expect(wrapper.get('label[for="template-input-research-and-verify-topic"]').text()).toContain('*');
    expect(wrapper.get('label[for="template-input-research-and-verify-notes"]').text()).not.toContain('*');
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
    expect(wrapper.get('#template-input-research-and-verify-topic-error').text()).toContain('Enter topic or question');
  });

  it('enables create and emits input values after required fields are entered', async () => {
    const wrapper = mountSelection();

    await wrapper.get('input#template-input-research-and-verify-topic').setValue('Compare GraphRAG and BM25');
    expect(wrapper.get('button').attributes('disabled')).toBeUndefined();

    await wrapper.get('textarea#template-input-research-and-verify-notes').setValue('Prefer cited sources.');
    await wrapper.get('form[aria-label="Create template pipeline"]').trigger('submit');

    expect(wrapper.emitted('create')).toEqual([[
      { topic: 'Compare GraphRAG and BM25', notes: 'Prefer cited sources.' },
    ]]);
  });

  it('resolves template input placeholders during instantiation', () => {
    const result = instantiateTemplate(baseTemplate, {
      primaryAgent: 'copilot',
      criticAgent: 'critic',
      inputs: { topic: 'Investigate template inputs', notes: '' },
    });

    expect(result.definition.nodes[0]).toMatchObject({
      acpClient: 'copilot',
      prompt: 'Investigate template inputs',
      displayLabel: 'Investigate Topic',
    });
  });
});
