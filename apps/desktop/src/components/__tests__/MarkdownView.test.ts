import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import MarkdownView from '../MarkdownView.vue';
import { useKnowledgeStore } from '../../stores/knowledge';
import { getMarkdownIt } from '../../composables/useMarkdownIt';

async function settle(): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await flushPromises();
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 10));
  }
}

async function mountMarkdown(content: unknown) {
  setActivePinia(createPinia());
  const wrapper = mount(MarkdownView, {
    props: { content: content as string },
    attachTo: document.body,
  });
  await settle();
  return wrapper;
}

describe('MarkdownView rendering regression coverage', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  describe('SVG sanitization in markdown', () => {
    it('svg with <script> child stripped', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><circle cx="5" cy="5" r="4" /><script>alert(1)</script></svg>',
      );

      expect(wrapper.find('svg').exists()).toBe(true);
      expect(wrapper.find('circle').exists()).toBe(true);
      expect(wrapper.find('script').exists()).toBe(false);
      expect(wrapper.html()).not.toContain('&lt;script');
      wrapper.unmount();
    });

    it('svg with onload handler stripped', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><circle cx="5" cy="5" r="4" /></svg>',
      );

      const svg = wrapper.find('svg');
      expect(svg.exists()).toBe(true);
      expect(svg.attributes('onload')).toBeUndefined();
      wrapper.unmount();
    });

    it('svg with onerror handler stripped', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><image href="https://example.com/icon.png" onerror="alert(1)" /></svg>',
      );

      const image = wrapper.find('image');
      expect(image.exists()).toBe(true);
      expect(image.attributes('onerror')).toBeUndefined();
      wrapper.unmount();
    });

    it('svg <foreignObject> with script content stripped', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><foreignObject><div>Label<script>alert(1)</script></div></foreignObject></svg>',
      );

      expect(wrapper.find('svg').exists()).toBe(true);
      expect(wrapper.find('foreignObject, foreignobject').exists()).toBe(true);
      expect(wrapper.text()).toContain('Label');
      expect(wrapper.find('script').exists()).toBe(false);
      wrapper.unmount();
    });

    it('svg with javascript: href stripped', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><a href="javascript:alert(1)"><text>Click</text></a></svg>',
      );

      expect(wrapper.find('svg').exists()).toBe(true);
      const link = wrapper.find('a');
      expect(link.exists()).toBe(true);
      expect(link.attributes('href') ?? '').not.toContain('javascript:');
      wrapper.unmount();
    });

    it('svg with data:text/html href stripped', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><a href="data:text/html;base64,PHNjcmlwdD4="><text>Click</text></a></svg>',
      );

      expect(wrapper.find('svg').exists()).toBe(true);
      const link = wrapper.find('a');
      expect(link.exists()).toBe(true);
      expect(link.attributes('href') ?? '').not.toContain('data:text/html');
      wrapper.unmount();
    });

    it('data:image/svg+xml nested script-bearing SVG blocked from href', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><use href="data:image/svg+xml;base64,PHN2ZyBvbmxvYWQ9YWxlcnQoMSk+PC9zdmc+" /></svg>',
      );

      const use = wrapper.find('use');
      expect(use.exists()).toBe(true);
      expect(use.attributes('href') ?? '').not.toContain('data:image/svg+xml');
      expect(use.attributes('xlink:href') ?? '').not.toContain('data:image/svg+xml');
      wrapper.unmount();
    });

    it('svg with internal xlink:href preserved', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><defs><path id="shape" d="M0 0 L10 10" /></defs><use xlink:href="#shape"></use></svg>',
      );

      const use = wrapper.find('use');
      expect(use.exists()).toBe(true);
      expect(use.attributes('xlink:href') ?? use.attributes('href')).toBe('#shape');
      wrapper.unmount();
    });

    it('foreignObject script is stripped while label text remains', async () => {
      const wrapper = await mountMarkdown(
        '<svg xmlns="http://www.w3.org/2000/svg"><foreignObject><script>alert(1)</script>label text</foreignObject></svg>',
      );

      expect(wrapper.find('foreignObject, foreignobject').exists()).toBe(true);
      expect(wrapper.find('script').exists()).toBe(false);
      expect(wrapper.text()).toContain('label text');
      expect(wrapper.html()).not.toContain('alert(1)');
      wrapper.unmount();
    });
  });

  describe('safe HTML allowlist', () => {
    it.each([
      ['<b> bold text renders', '<b>bold text</b>', 'b', 'bold text'],
      ['<i> italic text renders', '<i>italic text</i>', 'i', 'italic text'],
      ['<em> emphasis renders', '<em>emphasis</em>', 'em', 'emphasis'],
      ['<strong> renders', '<strong>strong text</strong>', 'strong', 'strong text'],
      ['<a href> link renders', '<a href="https://example.com">Example</a>', 'a[href="https://example.com"]', 'Example'],
      ['<code> inline code renders', '<code>const answer = 42;</code>', 'code', 'const answer = 42;'],
      ['<pre> preformatted renders', '<pre>line 1\nline 2</pre>', 'pre', 'line 1\nline 2'],
      ['<ul><li> list renders', '<ul><li>alpha</li><li>beta</li></ul>', 'ul li', 'alpha'],
      ['<ol><li> numbered list renders', '<ol><li>first</li><li>second</li></ol>', 'ol li', 'first'],
      ['<table><tr><td> table renders', '<table><tr><td>one</td></tr></table>', 'table td', 'one'],
      ['<blockquote> renders', '<blockquote>quoted</blockquote>', 'blockquote', 'quoted'],
    ])('%s', async (_name, source, selector, text) => {
      const wrapper = await mountMarkdown(source);

      const node = wrapper.find(selector);
      expect(node.exists()).toBe(true);
      expect(node.text()).toContain(text);
      wrapper.unmount();
    });

    it('empty allowlisted tag does not crash', async () => {
      for (const value of [undefined, null, '', '   ', '<b></b>']) {
        const wrapper = await mountMarkdown(value);
        expect(wrapper.find('article').exists()).toBe(true);
        if (value === '<b></b>') {
          expect(wrapper.find('b').exists()).toBe(true);
        }
        wrapper.unmount();
      }
    });
  });

  describe('disallowed HTML is stripped instead of escaped', () => {
    it.each([
      ['<script> stripped', '<p>safe</p><script>alert(1)</script>', 'script', '&lt;script'],
      ['<iframe> stripped', '<p>safe</p><iframe src="https://example.com"></iframe>', 'iframe', '&lt;iframe'],
      ['<object> stripped', '<p>safe</p><object data="https://example.com/payload"></object>', 'object', '&lt;object'],
      ['<embed> stripped', '<p>safe</p><embed src="https://example.com/payload">', 'embed', '&lt;embed'],
      ['<form> stripped', '<p>safe</p><form action="/submit"><input name="q" /></form>', 'form', '&lt;form'],
    ])('%s', async (_name, source, forbiddenSelector, escapedTag) => {
      const wrapper = await mountMarkdown(source);

      expect(wrapper.find('p').exists()).toBe(true);
      expect(wrapper.find(forbiddenSelector).exists()).toBe(false);
      expect(wrapper.html()).not.toContain(escapedTag);
      wrapper.unmount();
    });
  });

  describe('dangerous attributes are stripped', () => {
    it('javascript: href sanitized', async () => {
      const wrapper = await mountMarkdown('<a href="javascript:alert(1)">Click me</a>');

      const link = wrapper.find('a');
      expect(link.exists()).toBe(true);
      expect(link.attributes('href') ?? '').not.toContain('javascript:');
      wrapper.unmount();
    });

    it('data:text/html href stripped', async () => {
      const wrapper = await mountMarkdown('<a href="data:text/html;base64,PHNjcmlwdD4=">Click me</a>');

      const link = wrapper.find('a');
      expect(link.exists()).toBe(true);
      expect(link.attributes('href') ?? '').not.toContain('data:text/html');
      wrapper.unmount();
    });

    it('onclick handler stripped', async () => {
      const wrapper = await mountMarkdown('<p onclick="alert(1)">Click me</p>');

      const paragraph = wrapper.find('p');
      expect(paragraph.exists()).toBe(true);
      expect(paragraph.attributes('onclick')).toBeUndefined();
      wrapper.unmount();
    });

    it('onmouseover handler stripped', async () => {
      const wrapper = await mountMarkdown('<a href="https://example.com" onmouseover="alert(1)">Hover me</a>');

      const link = wrapper.find('a');
      expect(link.exists()).toBe(true);
      expect(link.attributes('onmouseover')).toBeUndefined();
      wrapper.unmount();
    });

    it('onerror on img stripped', async () => {
      const wrapper = await mountMarkdown('<img src="https://example.com/image.png" onerror="alert(1)" alt="demo" />');

      const image = wrapper.find('img');
      expect(image.exists()).toBe(true);
      expect(image.attributes('onerror')).toBeUndefined();
      wrapper.unmount();
    });

    it('ontoggle|onpointer|onanimation handlers stripped by hook', async () => {
      const wrapper = await mountMarkdown(
        '<details ontoggle="alert(1)"><summary>More</summary><p onpointerover="alert(2)" onanimationstart="alert(3)">Body</p></details>',
      );

      const details = wrapper.find('details');
      const paragraph = wrapper.find('p');
      expect(details.exists()).toBe(true);
      expect(paragraph.exists()).toBe(true);
      expect(details.attributes('ontoggle')).toBeUndefined();
      expect(paragraph.attributes('onpointerover')).toBeUndefined();
      expect(paragraph.attributes('onanimationstart')).toBeUndefined();
      wrapper.unmount();
    });

    it('mixed-case and custom on-prefixed attributes are stripped for safety', async () => {
      const wrapper = await mountMarkdown('<p OnLoad="alert(1)" ONERROR="alert(2)" onbehalfof="user">Body</p>');

      const paragraph = wrapper.find('p');
      expect(paragraph.exists()).toBe(true);
      expect(paragraph.attributes('onload')).toBeUndefined();
      expect(paragraph.attributes('onerror')).toBeUndefined();
      expect(paragraph.attributes('onbehalfof')).toBeUndefined();
      wrapper.unmount();
    });
  });

  describe('regression guards', () => {
    it('mermaid code fence still renders diagram', async () => {
      const wrapper = await mountMarkdown('```mermaid\ngraph TD\n  A-->B\n```');
      const mermaidResult = wrapper.find('.mermaid-rendered, pre[data-mermaid-processed="true"]');

      expect(mermaidResult.exists()).toBe(true);
      wrapper.unmount();
    });

    it('WikiLink [[concept]] still renders as pill', async () => {
      const wrapper = await mountMarkdown('Open [[Shared Concept]].');
      await wrapper.find('.wiki-pill').trigger('click');

      const store = useKnowledgeStore();
      expect(store.activeTabUri).toBe('memory://Shared Concept');
      expect(store.openTabs).toContain('memory://Shared Concept');
      wrapper.unmount();
    });

    it('syntax highlighted code fence still renders', async () => {
      const wrapper = await mountMarkdown('```ts\nconst answer: number = 42;\n```');

      expect(wrapper.find('pre.shiki').exists()).toBe(true);
      expect(wrapper.find('code span').exists()).toBe(true);
      wrapper.unmount();
    });
  });
});
