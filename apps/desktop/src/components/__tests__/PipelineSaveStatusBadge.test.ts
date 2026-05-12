// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import PipelineSaveStatusBadge from '../PipelineSaveStatusBadge.vue';
import type { PipelineSavingState } from '../../stores/pipeline-design';

function mountBadge(options?: Partial<{
  designMode: boolean;
  dirty: boolean;
  hasDraftNodes: boolean;
  savingState: PipelineSavingState;
}>) {
  return mount(PipelineSaveStatusBadge, {
    props: {
      designMode: options?.designMode ?? true,
      dirty: options?.dirty ?? true,
      hasDraftNodes: options?.hasDraftNodes ?? true,
      pipelineName: 'Draft pipeline',
      savingState: options?.savingState ?? 'idle',
    },
  });
}

describe('PipelineSaveStatusBadge', () => {
  it('shows dirty draft status and emits save', async () => {
    const wrapper = mountBadge();

    expect(wrapper.text()).toContain('Draft pipeline');
    expect(wrapper.text()).toContain('Unsaved changes');
    expect(wrapper.find('[aria-label="Unsaved draft"]').exists()).toBe(true);

    await wrapper.get('button').trigger('click');
    expect(wrapper.emitted('save')).toHaveLength(1);
  });

  it('disables save and hides dirty badge when a draft has no nodes', () => {
    const wrapper = mountBadge({ dirty: false, hasDraftNodes: false });

    expect(wrapper.text()).not.toContain('Unsaved changes');
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
  });

  it('renders saving, saved, and error states', async () => {
    const wrapper = mountBadge({ savingState: 'saving' });
    expect(wrapper.text()).toContain('Saving…');
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();

    await wrapper.setProps({ savingState: 'saved', designMode: false, dirty: false });
    expect(wrapper.text()).toContain('Saved');

    await wrapper.setProps({ savingState: 'error' });
    expect(wrapper.text()).toContain('Error');
  });
});
