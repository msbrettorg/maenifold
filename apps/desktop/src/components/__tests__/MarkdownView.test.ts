import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { defineComponent, h, nextTick } from 'vue';
import MarkdownView from '../MarkdownView.vue';
import { useKnowledgeStore } from '../../stores/knowledge';

const markdownFixture = [
  '# Shared Renderer',
  '',
  'Visit [[Shared Concept]] and https://example.com.',
  '',
  '```ts',
  'const answer: number = 42;',
  '```',
  '',
  '```mermaid',
  'graph TD',
  '  A-->B',
  '```',
  '',
  '- [ ] open task',
  '- [x] closed task',
  '',
  '| A | B |',
  '| - | - |',
  '| 1 | 2 |',
  '',
  'Footnote marker[^1].',
  '',
  '[^1]: Footnote body.',
].join('\n');

const consumerNames = ['chat', 'thinking', 'workflow-step', 'prompt-assembly'] as const;

function makeConsumer(name: string) {
  return defineComponent({
    name: `${name}-consumer`,
    props: {
      content: {
        type: String,
        required: true,
      },
    },
    setup(props) {
      return () => h('section', { 'data-consumer': name }, [
        h(MarkdownView, { content: props.content }),
      ]);
    },
  });
}

async function waitForRenderedMarkdown(wrapper: ReturnType<typeof mount>): Promise<void> {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 20));
    if (wrapper.find('.wiki-pill').exists()) return;
  }
}

async function mountConsumer(name: string, content = markdownFixture) {
  setActivePinia(createPinia());
  const wrapper = mount(makeConsumer(name), {
    props: { content },
    attachTo: document.body,
  });
  await waitForRenderedMarkdown(wrapper);
  return wrapper;
}

describe('MarkdownView', () => {
  it('renders identical markdown output for every markdown consumer', async () => {
    const htmlByConsumer = await Promise.all(
      consumerNames.map(async (name) => {
        const wrapper = await mountConsumer(name);
        const html = wrapper.findComponent(MarkdownView).html();
        wrapper.unmount();
        return html;
      }),
    );

    expect(new Set(htmlByConsumer).size).toBe(1);
  });

  it('navigates WikiLinks from every markdown consumer', async () => {
    for (const name of consumerNames) {
      const wrapper = await mountConsumer(name, 'Open [[Shared Concept]].');
      await wrapper.find('.wiki-pill').trigger('click');

      const store = useKnowledgeStore();
      expect(store.activeTabUri).toBe('memory://Shared Concept');
      expect(store.openTabs).toContain('memory://Shared Concept');
      wrapper.unmount();
    }
  });

  it('post-processes Mermaid fences in every markdown consumer', async () => {
    for (const name of consumerNames) {
      const wrapper = await mountConsumer(name, '```mermaid\ngraph TD\n  A-->B\n```');
      const mermaidResult = wrapper.find('.mermaid-rendered, pre[data-mermaid-processed="true"]');

      expect(mermaidResult.exists()).toBe(true);
      wrapper.unmount();
    }
  });

  it('syntax-highlights fenced code with a known language', async () => {
    const wrapper = await mountConsumer('chat', '```ts\nconst answer: number = 42;\n```');

    expect(wrapper.find('pre.shiki').exists()).toBe(true);
    expect(wrapper.find('code span').exists()).toBe(true);
    wrapper.unmount();
  });

  it('supports task lists, footnotes, GFM tables, and autolinks', async () => {
    const wrapper = await mountConsumer('chat', [
      '- [ ] open task',
      '- [x] closed task',
      '',
      '| A | B |',
      '| - | - |',
      '| 1 | 2 |',
      '',
      'Autolink https://example.com.',
      '',
      'Footnote marker[^1].',
      '',
      '[^1]: Footnote body.',
    ].join('\n'));

    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(2);
    expect(wrapper.find('table').exists()).toBe(true);
    expect(wrapper.find('a[href="https://example.com"]').exists()).toBe(true);
    expect(wrapper.find('.footnotes, .footnote-ref').exists()).toBe(true);
    wrapper.unmount();
  });
});
