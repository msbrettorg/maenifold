// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import SamplingApprovalModal from '../SamplingApprovalModal.vue';
import type { SamplingRequest } from '../../stores/sampling';

function request(): SamplingRequest {
  return {
    id: 'sampling-1',
    serverName: 'research-server',
    modelHint: 'gpt-4.1',
    messages: [
      { role: 'system', text: 'Use reliable sources.' },
      { role: 'user', text: 'Summarize the architecture memo.' },
    ],
    systemPrompt: 'Be precise.',
    maxTokens: 1024,
    queuedAt: 1_700_000_000_000,
  };
}

function mountModal() {
  return mount(SamplingApprovalModal, {
    props: { request: request() },
    global: {
      stubs: {
        Dialog: { template: '<section><slot /></section>' },
        DialogContent: { template: '<div><slot /></div>' },
        DialogFooter: { template: '<footer><slot /></footer>' },
        DialogHeader: { template: '<header><slot /></header>' },
        DialogTitle: { template: '<h2><slot /></h2>' },
        Button: { template: '<button type="button"><slot /></button>' },
        Select: defineComponent({
          name: 'Select',
          props: { modelValue: String },
          emits: ['update:modelValue'],
          template: '<div data-testid="model-select"><slot /></div>',
        }),
        SelectContent: { template: '<div><slot /></div>' },
        SelectItem: { props: ['value'], template: '<div><slot /></div>' },
        SelectTrigger: { template: '<button type="button"><slot /></button>' },
        SelectValue: { template: '<span><slot /></span>' },
      },
    },
  });
}

describe('SamplingApprovalModal', () => {
  it('renders_request_message_and_model_hint', () => {
    const wrapper = mountModal();

    expect(wrapper.text()).toContain('Summarize the architecture memo.');
    expect(wrapper.text()).toContain('gpt-4.1');
  });

  it('clicking_allow_once_emits_resolve_with_allow_once_action', async () => {
    const wrapper = mountModal();

    await wrapper.findAll('button').find((button) => button.text() === 'Allow Once')!.trigger('click');

    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'allow_once' }]);
  });

  it('clicking_allow_always_emits_resolve_with_allow_always_action', async () => {
    const wrapper = mountModal();

    await wrapper.findAll('button').find((button) => button.text() === 'Allow Always')!.trigger('click');

    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'allow_always' }]);
  });

  it('clicking_deny_emits_resolve_with_deny_action', async () => {
    const wrapper = mountModal();

    await wrapper.findAll('button').find((button) => button.text() === 'Deny')!.trigger('click');

    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'deny' }]);
  });

  it('model_override_select_updates_selectedModel_in_emitted_decision', async () => {
    const wrapper = mountModal();
    const select = wrapper.getComponent({ name: 'Select' });

    await select.vm.$emit('update:modelValue', 'gpt-4.1-mini');
    await wrapper.findAll('button').find((button) => button.text() === 'Allow Once')!.trigger('click');

    expect(wrapper.emitted('resolve')?.[0]).toEqual([
      { action: 'allow_once', selectedModel: 'gpt-4.1-mini' },
    ]);
  });
});
