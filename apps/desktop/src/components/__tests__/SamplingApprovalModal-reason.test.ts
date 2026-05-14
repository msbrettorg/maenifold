// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import SamplingApprovalModal from '../SamplingApprovalModal.vue';
import type { SamplingRequest } from '../../stores/sampling';

function request(overrides: Partial<SamplingRequest> = {}): SamplingRequest {
  return {
    id: 'sampling-reason-1',
    serverName: 'research-server',
    modelHint: 'gpt-4.1',
    messages: [{ role: 'user', text: 'Summarize the architecture memo.' }],
    systemPrompt: 'Be precise.',
    maxTokens: 1024,
    queuedAt: 1_700_000_000_000,
    ...overrides,
  };
}

function mountModal(overrides: Partial<SamplingRequest> = {}) {
  return mount(SamplingApprovalModal, {
    props: { request: request(overrides) },
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

describe('SamplingApprovalModal reason copy', () => {
  it('renders default copy when reason is undefined', () => {
    const wrapper = mountModal({ reason: undefined });

    expect(wrapper.text()).toContain('Approve sampling request');
    expect(wrapper.text()).toContain('wants to sample a model response');
  });

  it('renders default copy when reason is newRequest', () => {
    const wrapper = mountModal({ reason: { kind: 'newRequest' } });

    expect(wrapper.text()).toContain('Approve sampling request');
    expect(wrapper.text()).toContain('wants to sample a model response');
  });

  it('renders budget-exceeded copy when reason is budgetExceeded', () => {
    const wrapper = mountModal({
      reason: { kind: 'budgetExceeded', used: 10, budget: 10 },
    });

    expect(wrapper.text()).toContain('Tool-call budget exceeded');
    expect(wrapper.text()).toContain('reached its tool-call budget (10/10)');
  });
});
