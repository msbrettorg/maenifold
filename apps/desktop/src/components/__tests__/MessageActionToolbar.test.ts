// @vitest-environment jsdom
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import MessageActionToolbar from '../MessageActionToolbar.vue';
import { toast } from 'vue-sonner';

vi.mock('vue-sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

function mountToolbar(role: 'user' | 'assistant', disabled = false): VueWrapper {
  return mount(MessageActionToolbar, {
    attachTo: document.body,
    props: {
      messageRole: role,
      messageText: 'hello **markdown**',
      disabled,
    },
  });
}

function bodyButton(label: string): HTMLButtonElement {
  const match = Array.from(document.body.querySelectorAll('button')).find((button) =>
    button.textContent?.includes(label)
  );
  if (!(match instanceof HTMLButtonElement)) throw new Error(`Missing button: ${label}`);
  return match;
}

describe('MessageActionToolbar', () => {
  beforeEach(() => {
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      configurable: true,
    });
  });

  afterEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
  });

  it('renders Copy and Regenerate for assistant messages', () => {
    const wrapper = mountToolbar('assistant');

    expect(wrapper.findAll('button')).toHaveLength(2);
    wrapper.get('button[aria-label="Copy message"]');
    wrapper.get('button[aria-label="Regenerate from here"]');
    expect(wrapper.find('button[aria-label="Edit and resubmit"]').exists()).toBe(false);
  });

  it('renders Copy, Regenerate, and Edit for user messages', () => {
    const wrapper = mountToolbar('user');

    expect(wrapper.findAll('button')).toHaveLength(3);
    wrapper.get('button[aria-label="Copy message"]');
    wrapper.get('button[aria-label="Regenerate from here"]');
    wrapper.get('button[aria-label="Edit and resubmit"]');
  });

  it('copies message markdown and emits copy', async () => {
    const wrapper = mountToolbar('user');

    await wrapper.get('button[aria-label="Copy message"]').trigger('click');
    await flushPromises();

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('hello **markdown**');
    expect(toast.success).toHaveBeenCalledWith('Copied message to clipboard.');
    expect(wrapper.emitted('copy')).toEqual([[]]);
  });

  it('confirms before emitting regenerate', async () => {
    const wrapper = mountToolbar('assistant');

    await wrapper.get('button[aria-label="Regenerate from here"]').trigger('click');
    await flushPromises();

    expect(document.body.textContent).toContain(
      'Discard the messages after this one and regenerate? This cannot be undone.'
    );
    await bodyButton('Discard').click();
    await flushPromises();

    expect(wrapper.emitted('regenerate')).toEqual([[]]);
  });

  it('does not emit regenerate when confirmation is cancelled', async () => {
    const wrapper = mountToolbar('assistant');

    await wrapper.get('button[aria-label="Regenerate from here"]').trigger('click');
    await flushPromises();
    await bodyButton('Cancel').click();
    await flushPromises();

    expect(wrapper.emitted('regenerate')).toBeUndefined();
  });

  it('emits edit with the message text for user messages', async () => {
    const wrapper = mountToolbar('user');

    await wrapper.get('button[aria-label="Edit and resubmit"]').trigger('click');

    expect(wrapper.emitted('edit')).toEqual([[{ text: 'hello **markdown**' }]]);
  });

  it('disables every action when disabled', () => {
    const wrapper = mountToolbar('user', true);

    for (const button of wrapper.findAll('button')) {
      expect(button.attributes('disabled')).toBeDefined();
    }
  });
});
