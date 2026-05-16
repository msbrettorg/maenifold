import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import MarkdownView from '../MarkdownView.vue';
import { useAppShellStore } from '../../stores/app-shell';
import { useKnowledgeStore } from '../../stores/knowledge';
import { getMarkdownIt } from '../../composables/useMarkdownIt';
import { ToolEnvelopeError } from '../../composables/pipeline-mcp-transport';

interface MarkdownViewExposed {
  markUnresolvedWikilinks: () => Promise<void>;
  wikilinkExistsCache: Map<string, boolean>;
}

const memoryApi = vi.hoisted(() => ({
  readMemory: vi.fn(),
  writeMemory: vi.fn(),
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

async function mountMarkdown(content: unknown) {
  setActivePinia(createPinia());
  const wrapper = mount(MarkdownView, {
    props: { content: content as string },
    attachTo: document.body,
  });
  await settle();
  return wrapper;
}

function markdownViewExposed(wrapper: Awaited<ReturnType<typeof mountMarkdown>>): MarkdownViewExposed {
  return wrapper.vm as unknown as MarkdownViewExposed;
}

describe('MarkdownView rendering regression coverage', () => {
  beforeAll(async () => {
    await getMarkdownIt();
  });

  beforeEach(() => {
    document.body.innerHTML = '';
    memoryApi.readMemory.mockResolvedValue({
      uri: 'memory://Shared Concept',
      title: 'Shared Concept',
      content: '[[Shared Concept]]',
      checksum: 'cs1',
      created: null,
      modified: null,
    });
    memoryApi.writeMemory.mockResolvedValue('memory://Created');
    memoryApi.listMemories.mockResolvedValue('');
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
      await settle();

      const store = useKnowledgeStore();
      expect(store.activeTabUri).toBe('memory://Shared Concept');
      expect(store.openTabs).toContain('memory://Shared Concept');
      wrapper.unmount();
    });


    it('memory URI markdown link opens the note and switches to Knowledge', async () => {
      const wrapper = await mountMarkdown('[Open memory](memory://foo)');
      const knowledgeStore = useKnowledgeStore();
      const appShellStore = useAppShellStore();
      const openNote = vi.spyOn(knowledgeStore, 'openNote');
      const setActiveTab = vi.spyOn(appShellStore, 'setActiveTab');

      await wrapper.find('a[href="memory://foo"]').trigger('click');

      expect(openNote).toHaveBeenCalledWith('memory://foo');
      expect(setActiveTab).toHaveBeenCalledWith('knowledge');
      wrapper.unmount();
    });

    it('cmd-click memory URI markdown link opens the note in a new Knowledge tab', async () => {
      const wrapper = await mountMarkdown('[Open memory](memory://foo)');
      const knowledgeStore = useKnowledgeStore();
      const appShellStore = useAppShellStore();
      const openNote = vi.spyOn(knowledgeStore, 'openNote');
      const openInNewTab = vi.spyOn(knowledgeStore, 'openInNewTab');
      const setActiveTab = vi.spyOn(appShellStore, 'setActiveTab');

      await wrapper.find('a[href="memory://foo"]').trigger('click', { metaKey: true });

      expect(openInNewTab).toHaveBeenCalledWith('memory://foo');
      expect(openNote).not.toHaveBeenCalled();
      expect(setActiveTab).toHaveBeenCalledWith('knowledge');
      wrapper.unmount();
    });

    it('external markdown link does not route through the Knowledge store', async () => {
      const wrapper = await mountMarkdown('[External](https://example.com)');
      const knowledgeStore = useKnowledgeStore();
      const appShellStore = useAppShellStore();
      const openNote = vi.spyOn(knowledgeStore, 'openNote');
      const openInNewTab = vi.spyOn(knowledgeStore, 'openInNewTab');
      const setActiveTab = vi.spyOn(appShellStore, 'setActiveTab');

      await wrapper.find('a[href="https://example.com"]').trigger('click');

      expect(openNote).not.toHaveBeenCalled();
      expect(openInNewTab).not.toHaveBeenCalled();
      expect(setActiveTab).not.toHaveBeenCalled();
      wrapper.unmount();
    });

    it('DOMPurify preserves explicit memory URI markdown links', async () => {
      const wrapper = await mountMarkdown('[Memory link](memory://foo/bar)');
      const link = wrapper.find('a[href="memory://foo/bar"]');

      expect(link.exists()).toBe(true);
      expect(link.attributes('href')).toBe('memory://foo/bar');
      wrapper.unmount();
    });

    describe('T-322.5: bare memory:// autolink and edge cases', () => {
      it('bare memory://foo/bar text renders as clickable anchor', async () => {
        const wrapper = await mountMarkdown('memory://foo/bar');
        const link = wrapper.find('a[href="memory://foo/bar"]');

        expect(link.exists()).toBe(true);
        expect(link.attributes('href')).toBe('memory://foo/bar');
        wrapper.unmount();
      });

      it('click on empty-path memory:// is a no-op', async () => {
        const wrapper = await mountMarkdown('[empty](memory://)');
        const knowledgeStore = useKnowledgeStore();
        const openNote = vi.spyOn(knowledgeStore, 'openNote');
        const openInNewTab = vi.spyOn(knowledgeStore, 'openInNewTab');

        await wrapper.find('a[href="memory://"]').trigger('click');

        expect(openNote).not.toHaveBeenCalled();
        expect(openInNewTab).not.toHaveBeenCalled();
        wrapper.unmount();
      });

      it('bare memory://concepts/my-idea renders and routes correctly on click', async () => {
        const wrapper = await mountMarkdown('memory://concepts/my-idea');
        const knowledgeStore = useKnowledgeStore();
        const link = wrapper.find('a[href="memory://concepts/my-idea"]');
        const openNote = vi.spyOn(knowledgeStore, 'openNote');

        expect(link.exists()).toBe(true);
        expect(link.attributes('href')).toBe('memory://concepts/my-idea');

        await link.trigger('click');

        expect(openNote).toHaveBeenCalledWith('memory://concepts/my-idea');
        wrapper.unmount();
      });
    });

    it('unresolved WikiLink click opens the focused create dialog instead of opening a note', async () => {
      memoryApi.readMemory.mockRejectedValue(
        new ToolEnvelopeError('NOT_FOUND', 'Memory file not found: Missing Concept'),
      );
      const wrapper = await mountMarkdown('Open [[Missing Concept]].');
      const store = useKnowledgeStore();
      const openNote = vi.spyOn(store, 'openNote');

      await wrapper.find('.wiki-pill').trigger('click');
      await settle();

      expect(openNote).not.toHaveBeenCalled();
      expect(document.body.textContent).toContain('Create note for “Missing Concept”?');
      wrapper.unmount();
    });

    it('definitive not-found WikiLink check marks unresolved and caches false', async () => {
      memoryApi.readMemory.mockRejectedValue(
        new ToolEnvelopeError('NOT_FOUND', 'Memory file not found: ConceptX'),
      );
      const wrapper = await mountMarkdown('Open [[ConceptX]].');

      const chip = wrapper.find('[data-wikilink="ConceptX"]');
      expect(chip.attributes('data-unresolved')).toBe('true');
      expect(markdownViewExposed(wrapper).wikilinkExistsCache.get('ConceptX')).toBe(false);
      wrapper.unmount();
    });

    it('transient WikiLink check stays unknown and retries on next explicit check', async () => {
      memoryApi.readMemory.mockRejectedValue(new TypeError('fetch failed'));
      const wrapper = await mountMarkdown('Open [[ConceptX]].');

      const chip = wrapper.find('[data-wikilink="ConceptX"]');
      expect(chip.attributes('data-unresolved')).toBeUndefined();
      expect(markdownViewExposed(wrapper).wikilinkExistsCache.get('ConceptX')).toBeUndefined();

      memoryApi.readMemory.mockClear();
      await markdownViewExposed(wrapper).markUnresolvedWikilinks();

      expect(memoryApi.readMemory).toHaveBeenCalledTimes(1);
      expect(markdownViewExposed(wrapper).wikilinkExistsCache.get('ConceptX')).toBeUndefined();
      wrapper.unmount();
    });

    it('resolved WikiLink click preserves the existing openNote route', async () => {
      memoryApi.readMemory.mockResolvedValue({
        uri: 'memory://Existing Concept',
        title: 'Existing Concept',
        content: '[[Existing Concept]]',
        checksum: 'cs1',
        created: null,
        modified: null,
      });
      const wrapper = await mountMarkdown('Open [[Existing Concept]].');
      const store = useKnowledgeStore();
      const openNote = vi.spyOn(store, 'openNote');

      await wrapper.find('.wiki-pill').trigger('click');
      await settle();

      expect(openNote).toHaveBeenCalledWith('memory://Existing Concept');
      expect(document.body.textContent).not.toContain('Create note for “Existing Concept”?');
      wrapper.unmount();
    });

    it('limits post-render WikiLink existence checks to four in flight', async () => {
      let inFlight = 0;
      let maxInFlight = 0;
      memoryApi.readMemory.mockClear();
      memoryApi.readMemory.mockImplementation(async () => {
        inFlight += 1;
        maxInFlight = Math.max(maxInFlight, inFlight);
        await new Promise((resolve) => window.setTimeout(resolve, 10));
        inFlight -= 1;
        throw new ToolEnvelopeError('NOT_FOUND', 'Memory file not found: Concept');
      });

      const content = Array.from({ length: 12 }, (_unused, index) => `[[Concept${index}]]`).join(' ');
      const wrapper = await mountMarkdown(content);
      const chips = wrapper.findAll('[data-wikilink]');

      expect(memoryApi.readMemory).toHaveBeenCalledTimes(12);
      expect(maxInFlight).toBeLessThanOrEqual(4);
      expect(chips).toHaveLength(12);
      for (const chip of chips) {
        expect(chip.attributes('data-unresolved')).toBe('true');
      }
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
