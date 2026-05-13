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

  it('renders the turn-limit banner for max_turn_requests', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'max_turn_requests' } });

    expect(wrapper.text()).toContain('Turn limit reached.');
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

  it('renders the awaiting-tool banner for tool_use', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'tool_use' } });

    expect(wrapper.text()).toContain('Awaiting tool result.');
  });

  it('renders the error banner', () => {
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'error' } });

    expect(wrapper.text()).toContain('Error: see message.');
  });

  it('renders nothing for an unknown stop reason', () => {
    // Type system rejects unknown strings; this runtime safety check verifies the
    // default arm handles JSON-parsed values that bypass type narrowing.
    const wrapper = mount(StopReasonBanner, { props: { stopReason: 'unknown_reason' as unknown as 'error' } });

    expect(wrapper.text()).toBe('');
  });
});
