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

let currentVisibility: DocumentVisibilityState = 'visible';
const mountedWrappers: Array<ReturnType<typeof mount>> = [];

function setVisibilityState(visibility: DocumentVisibilityState): void {
  currentVisibility = visibility;
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    get: () => currentVisibility,
  });
}

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
  const wrapper = mount(SamplingApprovalModal, {
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
  mountedWrappers.push(wrapper);
  return wrapper;
}

function buttonByText(wrapper: ReturnType<typeof mount>, text: string) {
  const button = wrapper.findAll('button').find((candidate) => candidate.text() === text);
  expect(button, `expected ${text} button to be visible`).toBeDefined();
  return button!;
}

function expectManualButtons(wrapper: ReturnType<typeof mount>): void {
  expect(buttonByText(wrapper, 'Allow Once').exists()).toBe(true);
  expect(buttonByText(wrapper, 'Allow Always').exists()).toBe(true);
  expect(buttonByText(wrapper, 'Deny').exists()).toBe(true);
}

describe('SamplingApprovalModal visibility guarded countdown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.restoreAllMocks();
    setVisibilityState('visible');
    vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    configStore.config.samplingPreferences = { perServer: {} };
    configStore.setSamplingPreference.mockResolvedValue(undefined);
  });

  afterEach(() => {
    for (const wrapper of mountedWrappers.splice(0)) {
      wrapper.unmount();
    }
    setVisibilityState('visible');
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it('countdown_does_not_start_when_visibilityState_is_hidden', async () => {
    setVisibilityState('hidden');
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    await vi.advanceTimersByTimeAsync(2000);

    expect(wrapper.emitted('resolve')).toBeUndefined();
    expectManualButtons(wrapper);
  });

  it('countdown_does_not_start_when_window_lacks_focus', async () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false);
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    await vi.advanceTimersByTimeAsync(2000);

    expect(wrapper.emitted('resolve')).toBeUndefined();
    expectManualButtons(wrapper);
  });

  it('countdown_starts_when_visibility_changes_back_to_visible', async () => {
    setVisibilityState('hidden');
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    await vi.advanceTimersByTimeAsync(1000);
    expect(wrapper.emitted('resolve')).toBeUndefined();

    setVisibilityState('visible');
    document.dispatchEvent(new Event('visibilitychange'));
    await vi.advanceTimersByTimeAsync(1000);

    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'allow_once' }]);
  });

  it('countdown_cancels_when_visibility_changes_to_hidden_mid_countdown', async () => {
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    await vi.advanceTimersByTimeAsync(500);
    setVisibilityState('hidden');
    document.dispatchEvent(new Event('visibilitychange'));
    await vi.advanceTimersByTimeAsync(1000);

    expect(wrapper.emitted('resolve')).toBeUndefined();
    expectManualButtons(wrapper);
  });

  it('unmount_cleans_up_visibilitychange_listener', () => {
    const removeListenerSpy = vi.spyOn(document, 'removeEventListener');
    const wrapper = mountModal('foo', { foo: { alwaysAllow: true } });

    wrapper.unmount();
    mountedWrappers.pop();

    expect(removeListenerSpy).toHaveBeenCalledWith('visibilitychange', expect.any(Function));
  });
});
