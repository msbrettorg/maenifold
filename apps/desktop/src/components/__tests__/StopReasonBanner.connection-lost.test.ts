// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import StopReasonBanner from '../StopReasonBanner.vue';

describe('StopReasonBanner connection_lost', () => {
  it('renders connection-lost copy and emits reconnect', async () => {
    const wrapper = mount(StopReasonBanner, {
      props: { stopReason: 'connection_lost', canReconnect: true },
    });

    expect(wrapper.text()).toContain('Connection lost.');
    const button = wrapper.get('button[aria-label="Reconnect agent session"]');
    expect(button.text()).toBe('Reconnect');
    expect(button.attributes('disabled')).toBeUndefined();

    await button.trigger('click');

    expect(wrapper.emitted('reconnect')).toHaveLength(1);
  });

  it('disables reconnect when reconnect is unavailable', () => {
    const wrapper = mount(StopReasonBanner, {
      props: { stopReason: 'connection_lost', canReconnect: false },
    });

    expect(wrapper.get('button[aria-label="Reconnect agent session"]').attributes('disabled')).toBeDefined();
  });
});
