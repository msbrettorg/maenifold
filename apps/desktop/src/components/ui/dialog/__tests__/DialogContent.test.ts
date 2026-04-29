// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '..';

async function renderContent(contentClass?: string): Promise<string> {
  const wrapper = mount({
    components: { Dialog, DialogContent, DialogDescription, DialogTitle },
    template: `
      <Dialog :open="true">
        <DialogContent
          :class="contentClass"
          :disable-portal="true"
          :force-mount="true"
        >
          <DialogTitle>Test dialog</DialogTitle>
          <DialogDescription>DialogContent class regression fixture.</DialogDescription>
          Dialog body
        </DialogContent>
      </Dialog>
    `,
    setup() {
      return { contentClass };
    },
  }, {
    attachTo: document.body,
  });

  await nextTick();

  const content = wrapper.find('[data-slot="dialog-content"]');
  expect(content.exists()).toBe(true);
  const className = content.attributes('class') ?? '';
  wrapper.unmount();
  return className;
}

describe('DialogContent', () => {
  it('does not reintroduce the default sm width clamp when a base max width is supplied', async () => {
    const className = await renderContent('max-w-[80vw]');

    expect(className).toContain('max-w-[80vw]');
    expect(className).not.toContain('sm:max-w-md');
  });

  it('renders without the old default sm width clamp', async () => {
    const className = await renderContent();

    expect(className).not.toContain('sm:max-w-md');
  });

  it('preserves consumer supplied sm width classes', async () => {
    const className = await renderContent('sm:max-w-2xl');

    expect(className).toContain('sm:max-w-2xl');
  });
});
