import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMermaid } from '../useMermaid';

const renderMock = vi.hoisted(() => vi.fn());
const initializeMock = vi.hoisted(() => vi.fn());

vi.mock('mermaid', () => ({
  default: {
    initialize: initializeMock,
    render: renderMock,
  },
}));

function mountMermaidSource(source: string): HTMLElement {
  const container = document.createElement('div');
  container.innerHTML = `<pre><code class="language-mermaid">${source}</code></pre>`;
  document.body.append(container);
  return container;
}

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((fulfill) => { resolve = fulfill; });
  return { promise, resolve };
}

describe('useMermaid', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    renderMock.mockReset();
    renderMock.mockResolvedValue({
      svg: [
        '<svg xmlns="http://www.w3.org/2000/svg" role="img">',
        '<style>.nodeLabel { color: var(--text-primary); }</style>',
        '<foreignObject width="80" height="24"><div>Visible label</div></foreignObject>',
        '<text>Fallback label</text>',
        '<script>alert("xss")</script>',
        '</svg>',
      ].join(''),
    });
  });

  it('preserves Mermaid label rendering elements after sanitization', async () => {
    const container = mountMermaidSource('flowchart TD\n  A[Visible label] --> B[Done]');

    await useMermaid().renderMermaidBlocks(container);

    const rendered = container.querySelector('.mermaid-rendered');
    expect(rendered).not.toBeNull();

    const style = rendered?.querySelector('style');
    expect(style?.textContent?.trim()).toBeTruthy();

    const htmlLabel = Array.from(
      rendered?.querySelectorAll('foreignObject, foreignobject') ?? [],
    ).find((element) => element.textContent?.trim());
    expect(htmlLabel?.textContent?.trim()).toBeTruthy();
  });

  it('does not allow script elements in sanitized Mermaid output', async () => {
    const container = mountMermaidSource('stateDiagram-v2\n  [*] --> Ready');

    await useMermaid().renderMermaidBlocks(container);

    expect(container.querySelector('script')).toBeNull();
  });

  it('returns before rendering when continuation is already inactive', async () => {
    const container = mountMermaidSource('flowchart TD\n  A --> B');

    await useMermaid().renderMermaidBlocks(container, () => false);

    expect(renderMock).not.toHaveBeenCalled();
    expect(container.querySelector('pre')).not.toBeNull();
  });

  it('skips DOM replacement when continuation flips false after mermaid.render resolves', async () => {
    const container = mountMermaidSource('flowchart TD\n  A --> B');
    const started = createDeferred<void>();
    const rendered = createDeferred<{ svg: string }>();
    renderMock.mockImplementationOnce(async () => {
      started.resolve();
      await new Promise((resolve) => setTimeout(resolve, 0));
      return rendered.promise;
    });
    let active = true;
    const renderPromise = useMermaid().renderMermaidBlocks(container, () => active);

    await started.promise;
    active = false;
    rendered.resolve({ svg: '<svg xmlns="http://www.w3.org/2000/svg"><text>Rendered</text></svg>' });
    await renderPromise;

    expect(container.querySelector('.mermaid-rendered')).toBeNull();
    expect(container.querySelector('pre')).not.toBeNull();
  });

  describe('renderMermaidBlocks cancellation', () => {
    it('does NOT call preBlock.replaceWith when shouldContinue returns false after mermaid.render resolves', async () => {
      const container = document.createElement('div');
      container.innerHTML = '<pre><code class="language-mermaid">flowchart TD\n  A-->B</code></pre>';
      document.body.appendChild(container);
      const pre = container.querySelector('pre')!;
      const spy = vi.spyOn(pre, 'replaceWith');
      const deferred = createDeferred<{ svg: string }>();
      renderMock.mockImplementationOnce(async () => deferred.promise);
      const { renderMermaidBlocks } = useMermaid();
      let allow = true;
      const renderPromise = renderMermaidBlocks(container, () => allow);

      await new Promise((resolve) => setTimeout(resolve, 0));
      allow = false;
      deferred.resolve({ svg: '<svg xmlns="http://www.w3.org/2000/svg"/>' });
      await renderPromise;

      expect(spy).not.toHaveBeenCalled();
    });

    it('DOES call preBlock.replaceWith when shouldContinue returns true (falsifiability proof)', async () => {
      const container = document.createElement('div');
      container.innerHTML = '<pre><code class="language-mermaid">flowchart TD\n  X-->Y</code></pre>';
      document.body.appendChild(container);
      const pre = container.querySelector('pre')!;
      const spy = vi.spyOn(pre, 'replaceWith');
      const deferred = createDeferred<{ svg: string }>();
      renderMock.mockImplementationOnce(async () => deferred.promise);
      const { renderMermaidBlocks } = useMermaid();
      const renderPromise = renderMermaidBlocks(container, () => true);

      await new Promise((resolve) => setTimeout(resolve, 0));
      deferred.resolve({ svg: '<svg xmlns="http://www.w3.org/2000/svg"/>' });
      await renderPromise;

      expect(spy).toHaveBeenCalled();
    });
  });
});
