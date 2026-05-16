// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import PipelineActionBar from '../PipelineActionBar.vue';
import { usePipelineStore } from '../../stores/pipeline';
import type { PipelineDefinition } from '../../lib/pipeline-types-edge';

function mountCreateActionBar() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = usePipelineStore();
  store.designMode = true;
  store.currentPipelineId = null;
  store.currentView = null;
  store.localDefinition = {
    nodes: [{ id: 'draft-node', type: 'tool', toolName: 'example.tool' }],
    edges: [],
  } as PipelineDefinition;
  const createPipeline = vi.spyOn(store, 'createPipeline').mockResolvedValue({ pipelineId: 'created' });
  const wrapper = mount(PipelineActionBar, { global: { plugins: [pinia] } });
  return { createPipeline, wrapper };
}

describe('PipelineActionBar create form', () => {
  it('renders name and description fields in the design create flow', () => {
    const { wrapper } = mountCreateActionBar();

    expect(wrapper.get('label[for="pipeline-create-name"]').text()).toBe('Name');
    expect(wrapper.find('input#pipeline-create-name').exists()).toBe(true);
    expect(wrapper.get('label[for="pipeline-create-description"]').text()).toBe('Description');
    expect(wrapper.find('textarea#pipeline-create-description').exists()).toBe(true);
  });

  it('submits sanitized name and optional description to the create action', async () => {
    const { createPipeline, wrapper } = mountCreateActionBar();

    await wrapper.get('input#pipeline-create-name').setValue('Research Run');
    await wrapper.get('textarea#pipeline-create-description').setValue('Investigate the account pipeline');
    await wrapper.get('form[aria-label="Create pipeline details"]').trigger('submit');
    await flushPromises();

    expect(createPipeline).toHaveBeenCalledWith(undefined, {
      name: 'research-run',
      description: 'Investigate the account pipeline',
    });
  });

  it('falls back to the backend-generated ID and warns when name is empty', async () => {
    const { createPipeline, wrapper } = mountCreateActionBar();

    await wrapper.get('form[aria-label="Create pipeline details"]').trigger('submit');
    await flushPromises();

    expect(createPipeline).toHaveBeenCalledWith(undefined, {
      name: undefined,
      description: undefined,
    });
    expect(wrapper.get('#pipeline-create-name-help').text()).toContain('pipeline-* ID');
    expect(wrapper.get('#pipeline-create-name-help').classes()).toContain('text-danger');
  });
});
