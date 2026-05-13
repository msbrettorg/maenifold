// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import { mount, type VueWrapper } from '@vue/test-utils';
import HomeComposer from '../HomeComposer.vue';

function mountComposer(
  props: Partial<InstanceType<typeof HomeComposer>['$props']> = {},
): VueWrapper {
  return mount(HomeComposer, {
    props: {
      daemonOnline: true,
      isConnecting: false,
      submitPrompt: vi.fn().mockResolvedValue(true),
      ...props,
    },
  });
}

function enterEvent(options: { isComposing?: boolean; keyCode?: number; shiftKey?: boolean } = {}) {
  const event = new KeyboardEvent('keydown', {
    key: 'Enter',
    shiftKey: options.shiftKey ?? false,
    bubbles: true,
    cancelable: true,
  });
  Object.defineProperty(event, 'isComposing', {
    value: options.isComposing ?? false,
    configurable: true,
  });
  if (options.keyCode !== undefined) {
    Object.defineProperty(event, 'keyCode', {
      value: options.keyCode,
      configurable: true,
    });
  }
  const preventDefault = vi.spyOn(event, 'preventDefault');
  return { event, preventDefault };
}

async function dispatchEnter(wrapper: VueWrapper, event: KeyboardEvent): Promise<void> {
  wrapper.find('textarea').element.dispatchEvent(event);
  await nextTick();
}

describe('HomeComposer', () => {
  it('submits on plain Enter', async () => {
    const submitPrompt = vi.fn().mockResolvedValue(true);
    const wrapper = mountComposer({ submitPrompt });
    await wrapper.find('textarea').setValue('hello from home');
    const { event, preventDefault } = enterEvent();

    await dispatchEnter(wrapper, event);

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(submitPrompt).toHaveBeenCalledWith('hello from home');
  });

  it('does not submit on Shift+Enter', async () => {
    const submitPrompt = vi.fn().mockResolvedValue(true);
    const wrapper = mountComposer({ submitPrompt });
    await wrapper.find('textarea').setValue('line one');
    const { event, preventDefault } = enterEvent({ shiftKey: true });

    await dispatchEnter(wrapper, event);

    expect(preventDefault).not.toHaveBeenCalled();
    expect(submitPrompt).not.toHaveBeenCalled();
  });

  it('does not submit IME composition Enter', async () => {
    const submitPrompt = vi.fn().mockResolvedValue(true);
    const wrapper = mountComposer({ submitPrompt });
    await wrapper.find('textarea').setValue('composing');
    const composing = enterEvent({ isComposing: true });
    const legacy = enterEvent({ keyCode: 229 });

    await dispatchEnter(wrapper, composing.event);
    await dispatchEnter(wrapper, legacy.event);

    expect(composing.preventDefault).not.toHaveBeenCalled();
    expect(legacy.preventDefault).not.toHaveBeenCalled();
    expect(submitPrompt).not.toHaveBeenCalled();
  });

  it('does not submit whitespace-only input on Enter', async () => {
    const submitPrompt = vi.fn().mockResolvedValue(true);
    const wrapper = mountComposer({ submitPrompt });
    await wrapper.find('textarea').setValue('   \n  ');
    const { event, preventDefault } = enterEvent();

    await dispatchEnter(wrapper, event);

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(submitPrompt).not.toHaveBeenCalled();
  });

  it('does not submit when disabled', async () => {
    const submitPrompt = vi.fn().mockResolvedValue(true);
    const offlineWrapper = mountComposer({ daemonOnline: false, submitPrompt });
    await offlineWrapper.find('textarea').setValue('offline prompt');
    const offline = enterEvent();

    await dispatchEnter(offlineWrapper, offline.event);

    const connectingWrapper = mountComposer({ isConnecting: true, submitPrompt });
    await connectingWrapper.find('textarea').setValue('connecting prompt');
    const connecting = enterEvent();

    await dispatchEnter(connectingWrapper, connecting.event);

    expect(offline.preventDefault).toHaveBeenCalledTimes(1);
    expect(connecting.preventDefault).toHaveBeenCalledTimes(1);
    expect(submitPrompt).not.toHaveBeenCalled();
  });
});
