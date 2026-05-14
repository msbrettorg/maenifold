// @vitest-environment jsdom
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import MarkdownView from '../MarkdownView.vue';
import { getMarkdownIt } from '../../composables/useMarkdownIt';
import { useKnowledgeStore } from '../../stores/knowledge';

const memoryApi = vi.hoisted(() => ({
  readMemory: vi.fn(),
  listMemories: vi.fn(),
}));
vi.mock('../../composables/useMemoryApi', () => ({ useMemoryApi: () => memoryApi }));

async function settle(): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await flushPromises();
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 10));
  }
}

describe('WikiPill accessibility', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    setActivePinia(createPinia());
    document.body.innerHTML = '';
    memoryApi.readMemory.mockResolvedValue({
      uri: 'memory://Concept',
      title: 'Concept',
      content: '[[Concept]]',
      checksum: 'cs1',
      created: null,
      modified: null,
    });
    memoryApi.listMemories.mockResolvedValue('');
  });

  it('renders [[concept]] as an anchor element with implicit link role', async () => {
    const wrapper = mount(MarkdownView, {
      props: { content: 'Open [[Shared Concept]] here.' },
      attachTo: document.body,
    });
    await settle();

    const chip = wrapper.find('.wiki-pill');
    expect(chip.exists()).toBe(true);
    expect(chip.element.tagName).toBe('A');
    wrapper.unmount();
  });

  it('wiki-pill anchor has href attribute making it keyboard tab-focusable', async () => {
    const wrapper = mount(MarkdownView, {
      props: { content: '[[Focusable Concept]]' },
      attachTo: document.body,
    });
    await settle();

    const chip = wrapper.find('.wiki-pill');
    expect(chip.exists()).toBe(true);
    expect(chip.attributes('href')).toBeTruthy();
    wrapper.unmount();
  });

  it('Enter key equivalent (click) on wiki-pill navigates to the note', async () => {
    const wrapper = mount(MarkdownView, {
      props: { content: '[[Enter Concept]]' },
      attachTo: document.body,
    });
    await settle();

    const chip = wrapper.find('.wiki-pill');
    await chip.trigger('click');

    const store = useKnowledgeStore();
    expect(store.activeTabUri).toBe('memory://Enter Concept');
    wrapper.unmount();
  });

  it('Space keydown on wiki-pill navigates to the note', async () => {
    const wrapper = mount(MarkdownView, {
      props: { content: '[[Space Concept]]' },
      attachTo: document.body,
    });
    await settle();

    const chip = wrapper.find('.wiki-pill');
    await chip.trigger('keydown', { key: ' ' });

    const store = useKnowledgeStore();
    expect(store.activeTabUri).toBe('memory://Space Concept');
    wrapper.unmount();
  });
});
