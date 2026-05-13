// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent } from 'vue';
import SamplingApprovalModal from '../SamplingApprovalModal.vue';
import type { SamplingRequest } from '../../stores/sampling';

type SamplingPreference = { alwaysAllow: boolean; modelOverride?: string };

const { configStore, toast } = vi.hoisted(() => ({
  configStore: {
    config: {
      samplingPreferences: { perServer: {} as Record<string, SamplingPreference> },
    },
    setSamplingPreference: vi.fn(),
  },
  toast: {
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock('../../stores/config', () => ({
  useConfigStore: () => configStore,
}));

vi.mock('vue-sonner', () => ({ toast }));

function request(serverName: string): SamplingRequest {
  return {
    id: `sampling-${serverName}`,
    serverName,
    modelHint: 'gpt-4.1',
    messages: [{ role: 'user', text: `Sample for ${serverName}` }],
    systemPrompt: 'Be concise.',
    maxTokens: 512,
    queuedAt: 1_700_000_000_000,
  };
}

function mountModal(serverName: string, perServer: Record<string, SamplingPreference> = {}) {
  configStore.config.samplingPreferences = { perServer };
  return mount(SamplingApprovalModal, {
    props: { request: request(serverName) },
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

function buttonByText(wrapper: ReturnType<typeof mount>, text: string) {
  const button = wrapper.findAll('button').find((candidate) => candidate.text() === text);
  expect(button, `expected ${text} button to be visible`).toBeDefined();
  return button!;
}

describe('SamplingApprovalModal countdown preference consumption', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.restoreAllMocks();
    configStore.config.samplingPreferences = { perServer: {} };
    configStore.setSamplingPreference.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it('auto_approve_fires_after_1s_when_alwaysAllow_true', async () => {
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    buttonByText(wrapper, 'Cancel');
    await vi.advanceTimersByTimeAsync(1000);

    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'allow_once' }]);
  });

  it('cancel_during_countdown_stops_auto_approve', async () => {
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    await vi.advanceTimersByTimeAsync(500);
    await buttonByText(wrapper, 'Cancel').trigger('click');
    await vi.advanceTimersByTimeAsync(1000);

    expect(wrapper.emitted('resolve')).toBeUndefined();
    expect(buttonByText(wrapper, 'Allow Once').exists()).toBe(true);
    expect(buttonByText(wrapper, 'Allow Always').exists()).toBe(true);
    expect(buttonByText(wrapper, 'Deny').exists()).toBe(true);
  });

  it('manual_allow_always_persists_preference_via_helper', async () => {
    configStore.setSamplingPreference.mockReturnValue(new Promise(() => {}));
    const wrapper = mountModal('bar', { bar: { alwaysAllow: false } });

    await buttonByText(wrapper, 'Allow Always').trigger('click');

    expect(configStore.setSamplingPreference).toHaveBeenCalledWith('bar', { alwaysAllow: true });
    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'allow_always' }]);
  });

  it('manual_allow_always_resolution_proceeds_even_if_persist_throws', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    configStore.setSamplingPreference.mockRejectedValue(new Error('disk full'));
    const wrapper = mountModal('bar', { bar: { alwaysAllow: false } });

    await buttonByText(wrapper, 'Allow Always').trigger('click');
    await Promise.resolve();

    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'allow_always' }]);
    expect(configStore.setSamplingPreference).toHaveBeenCalledWith('bar', { alwaysAllow: true });
    expect(
      warnSpy.mock.calls.length + toast.warning.mock.calls.length + toast.error.mock.calls.length,
    ).toBeGreaterThan(0);
  });

  it('no_countdown_when_alwaysAllow_false_or_undefined', async () => {
    const withoutPreference = mountModal('foo');

    await vi.advanceTimersByTimeAsync(5000);

    expect(withoutPreference.emitted('resolve')).toBeUndefined();
    expect(buttonByText(withoutPreference, 'Allow Once').exists()).toBe(true);
    withoutPreference.unmount();

    const explicitlyFalse = mountModal('foo', { foo: { alwaysAllow: false } });
    await vi.advanceTimersByTimeAsync(5000);

    expect(explicitlyFalse.emitted('resolve')).toBeUndefined();
    expect(buttonByText(explicitlyFalse, 'Allow Always').exists()).toBe(true);
  });

  it('unmount_clears_active_timer', async () => {
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    buttonByText(wrapper, 'Cancel');
    await vi.advanceTimersByTimeAsync(500);
    wrapper.unmount();
    await vi.advanceTimersByTimeAsync(1000);

    expect(wrapper.emitted('resolve')).toBeUndefined();
  });
});
