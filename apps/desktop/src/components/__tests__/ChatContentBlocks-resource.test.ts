import { describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import ChatContentBlocks from '../ChatContentBlocks.vue';
import ResourcePreview from '../ResourcePreview.vue';
import { useMcpResources } from '../../composables/useMcpResources';

vi.mock('../../composables/useMcpResources', () => ({
  useMcpResources: vi.fn(),
}));

describe('ChatContentBlocks resource rendering', () => {
  it('renders resource blocks with ResourcePreview instead of a raw body pre', () => {
    setActivePinia(createPinia());
    const wrapper = mount(ChatContentBlocks, {
      props: {
        blocks: [{
          type: 'resource',
          resource: { uri: 'memory://note', mimeType: 'text/markdown', text: '# Note' },
        } as never],
      },
      attachTo: document.body,
    });

    expect(wrapper.findComponent(ResourcePreview).exists()).toBe(true);
    expect(wrapper.findComponent(ResourcePreview).props('dispatched')).toEqual({ kind: 'markdown', text: '# Note' });
    expect(wrapper.find('pre').exists()).toBe(false);
    wrapper.unmount();
  });

  it('loads a resource_link on click and renders the inline preview', async () => {
    setActivePinia(createPinia());
    const readResource = vi.fn().mockResolvedValue({
      uri: 'memory://linked',
      mimeType: 'application/json',
      text: '{"ok":true}',
    });
    vi.mocked(useMcpResources).mockReturnValue({
      readResource,
      subscribeResource: vi.fn(),
      unsubscribeResource: vi.fn(),
      listResources: vi.fn(),
      listResourceTemplates: vi.fn(),
    });

    const wrapper = mount(ChatContentBlocks, {
      props: {
        blocks: [{
          type: 'resource_link',
          uri: 'memory://linked',
          name: 'linked.json',
          mimeType: 'application/json',
        } as never],
      },
      attachTo: document.body,
    });

    await wrapper.find('[data-testid="resource-link-chip"]').trigger('click');
    await flushPromises();

    expect(readResource).toHaveBeenCalledWith('memory://linked');
    expect(wrapper.findComponent(ResourcePreview).exists()).toBe(true);
    expect(wrapper.findComponent(ResourcePreview).props('dispatched')).toEqual({
      kind: 'json',
      pretty: '{\n  "ok": true\n}',
    });
    wrapper.unmount();
  });

  it('surfaces resource_link read errors inline', async () => {
    setActivePinia(createPinia());
    vi.mocked(useMcpResources).mockReturnValue({
      readResource: vi.fn().mockRejectedValue(new Error('404 not found')),
      subscribeResource: vi.fn(),
      unsubscribeResource: vi.fn(),
      listResources: vi.fn(),
      listResourceTemplates: vi.fn(),
    });

    const wrapper = mount(ChatContentBlocks, {
      props: {
        blocks: [{
          type: 'resource_link',
          uri: 'memory://missing',
          name: 'missing.md',
          mimeType: 'text/markdown',
        } as never],
      },
      attachTo: document.body,
    });

    await wrapper.find('[data-testid="resource-link-chip"]').trigger('click');
    await flushPromises();

    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('404 not found');
    wrapper.unmount();
  });
});
