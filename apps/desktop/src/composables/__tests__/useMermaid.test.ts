import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMermaid } from '../useMermaid';

const renderMock = vi.hoisted(() => vi.fn());

vi.mock('mermaid', () => ({
  default: {
    initialize: vi.fn(),
    render: renderMock,
  },
}));

function mountMermaidSource(source: string): HTMLElement {
  const container = document.createElement('div');
  container.innerHTML = `<pre><code class="language-mermaid">${source}</code></pre>`;
  document.body.append(container);
  return container;
}

describe('useMermaid', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    renderMock.mockReset();
    renderMock.mockResolvedValue({
      svg: [
        '<svg xmlns="http://www.w3.org/2000/svg" role="img">',
        '<style>.nodeLabel { color: #111827; }</style>',
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
});
