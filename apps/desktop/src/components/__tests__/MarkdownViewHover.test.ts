import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import MarkdownView from '../MarkdownView.vue';
import { getMarkdownIt } from '../../composables/useMarkdownIt';

interface MarkdownViewHoverExposed {
  memoryPreviewCache: Map<string, { title: string; firstParagraph: string; modified: string | null }>;
  wikilinkExistsCache: Map<string, boolean>;
}

const memoryApi = vi.hoisted(() => ({
  readMemory: vi.fn(),
  writeMemory: vi.fn(),
  listMemories: vi.fn(),
}));
vi.mock('../../composables/useMemoryApi', () => ({ useMemoryApi: () => memoryApi }));

async function flushView(): Promise<void> {
  await flushPromises();
  await nextTick();
}

function hoverExposed(wrapper: Awaited<ReturnType<typeof mountMarkdown>>): MarkdownViewHoverExposed {
  return wrapper.vm as unknown as MarkdownViewHoverExposed;
}

async function mountMarkdown(content: string) {
  setActivePinia(createPinia());
  const wrapper = mount(MarkdownView, {
    props: { content },
    attachTo: document.body,
  });
  await flushView();
  return wrapper;
}

describe('MarkdownView memory hover previews', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = '';
    memoryApi.readMemory.mockReset();
    memoryApi.readMemory.mockResolvedValue({
      uri: 'memory://Shared Concept',
      title: 'Shared Concept',
      content: 'First paragraph.\n\nSecond paragraph.',
      checksum: 'checksum',
      created: null,
      modified: '2026-05-16T12:00:00.000Z',
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows a hover preview after the delay for explicit memory links', async () => {
    const wrapper = await mountMarkdown('[Open memory](memory://Shared%20Concept)');
    const link = wrapper.find('a[href="memory://Shared%20Concept"]');

    await link.trigger('mouseover');
    await vi.advanceTimersByTimeAsync(300);
    await flushView();

    expect(document.body.textContent).toContain('Shared Concept');
    expect(document.body.textContent).toContain('First paragraph.');
    wrapper.unmount();
  });

  it('hides the hover preview when leaving the trigger', async () => {
    const wrapper = await mountMarkdown('[Open memory](memory://Shared%20Concept)');
    const link = wrapper.find('a[href="memory://Shared%20Concept"]');

    await link.trigger('mouseover');
    await vi.advanceTimersByTimeAsync(300);
    await flushView();
    expect(document.body.textContent).toContain('First paragraph.');

    await link.trigger('mouseout');
    await flushView();

    expect(document.body.textContent).not.toContain('First paragraph.');
    wrapper.unmount();
  });

  it('reuses the shared cache for wikilink hover previews', async () => {
    const wrapper = await mountMarkdown('Open [[Shared Concept]].');
    const chip = wrapper.find('[data-wikilink="Shared Concept"]');

    await vi.advanceTimersByTimeAsync(60);
    await flushView();
    expect(memoryApi.readMemory).toHaveBeenCalledTimes(1);
    expect(hoverExposed(wrapper).wikilinkExistsCache.get('Shared Concept')).toBe(true);

    await chip.trigger('mouseover');
    await vi.advanceTimersByTimeAsync(300);
    await flushView();
    await chip.trigger('mouseout');
    await flushView();
    await chip.trigger('mouseover');
    await vi.advanceTimersByTimeAsync(300);
    await flushView();

    expect(memoryApi.readMemory).toHaveBeenCalledTimes(1);
    expect(hoverExposed(wrapper).memoryPreviewCache.get('Shared Concept')?.firstParagraph).toBe('First paragraph.');
    wrapper.unmount();
  });
});
