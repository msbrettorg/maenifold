// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import StopReasonBanner from '../StopReasonBanner.vue';

describe('StopReasonBanner', () => {
  it('renders nothing for end_turn', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'end_turn' } });

    expect(wrapper.text()).toBe('');
  });

  it('renders the token limit banner with AlertCircle icon', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'max_tokens' } });

    expect(wrapper.text()).toContain('Response truncated due to token limits.');
    expect(wrapper.find('.lucide-circle-alert').exists()).toBe(true);
  });

  it('renders the cancelled banner with XCircle icon', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'cancelled' } });

    expect(wrapper.text()).toContain('Cancelled.');
    expect(wrapper.find('.lucide-circle-x').exists()).toBe(true);
  });

  it('renders the refusal banner', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'refusal' } });

    expect(wrapper.text()).toContain('The agent refused this request.');
  });

  it('renders the error banner', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'error' } });

    expect(wrapper.text()).toContain('Error: see message.');
  });

  it('renders nothing for an unknown stop reason', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'unknown_reason' } });

    expect(wrapper.text()).toBe('');
  });
});
