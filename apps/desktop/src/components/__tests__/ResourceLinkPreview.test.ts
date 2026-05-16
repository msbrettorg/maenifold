// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import ResourceLinkPreview from '../ResourceLinkPreview.vue';
import type { ResourceLinkContentBlock } from '../../lib/types';

describe('ResourceLinkPreview', () => {
  it('renders the resource link name and visible URI', () => {
    const wrapper = mount(ResourceLinkPreview, {
      props: {
        block: {
          type: 'resource_link',
          name: 'Foo',
          uri: 'https://example.com/foo',
          mimeType: 'text/plain',
        } as ResourceLinkContentBlock,
      },
    });

    const chip = wrapper.find('[data-testid="resource-link-chip"]');
    expect(chip.exists()).toBe(true);
    expect(chip.text()).toContain('Foo');
    expect(chip.text()).toContain('https://example.com/foo');
  });
});
