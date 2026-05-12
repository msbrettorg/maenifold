// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import GatePipelineBrowser from '../GatePipelineBrowser.vue';

describe('GatePipelineBrowser', () => {
  it('renders the built-in gate pipelines and default state diagram', () => {
    const wrapper = mount(GatePipelineBrowser);

    expect(wrapper.text()).toContain('Gate process pipelines');
    expect(wrapper.text()).toContain('Spec Driven Development');
    expect(wrapper.text()).toContain('Test Driven Development');
    expect(wrapper.text()).toContain('Get Stuff Done');
    expect(wrapper.text()).toContain('Lightweight');
    expect(wrapper.text()).toContain('sdd · schema 1 · initial pending');
    expect(wrapper.text()).toContain('pass → red-team');
  });

  it('selects a pipeline and updates the read-only transitions', async () => {
    const wrapper = mount(GatePipelineBrowser);

    await wrapper.get('[data-pipeline-id="gsd"]').trigger('click');

    expect(wrapper.text()).toContain('gsd · schema 1 · initial pending');
    expect(wrapper.text()).toContain('verify');
    expect(wrapper.text()).toContain('fail → execute');
    expect(wrapper.get('[data-pipeline-id="gsd"]').attributes('aria-selected')).toBe('true');
    expect(wrapper.get('[data-pipeline-id="sdd"]').attributes('aria-selected')).toBe('false');
  });
});
