import type mermaidType from 'mermaid';
import type { Ref } from 'vue';

let mermaidPromise: Promise<typeof mermaidType> | null = null;
let mermaidRenderSequence = 0;

function getMermaidTheme(): 'dark' | 'default' {
  const isDark =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-color-scheme: dark)').matches;

  return isDark ? 'dark' : 'default';
}

async function getMermaid(): Promise<typeof mermaidType> {
  if (!mermaidPromise) {
    mermaidPromise = import('mermaid').then((module) => module.default);
  }

  const mermaid = await mermaidPromise;
  mermaid.initialize({
    startOnLoad: false,
    theme: getMermaidTheme(),
  });

  return mermaid;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll('\'', '&#39;');
}

function renderErrorMessageHtml(error: unknown): string {
  const message = error instanceof Error ? error.message : 'Unknown error';
  return `<div class="mermaid-error-message">Unable to render Mermaid diagram: ${escapeHtml(message)}</div>`;
}

function resolveContainer(
  containerOrRef: HTMLElement | Ref<HTMLElement | null>
): HTMLElement | null {
  if (containerOrRef instanceof HTMLElement) {
    return containerOrRef;
  }
  return containerOrRef.value;
}

export function useMermaid() {
  async function renderMermaidBlocks(
    containerOrRef: HTMLElement | Ref<HTMLElement | null>
  ): Promise<void> {
    const container = resolveContainer(containerOrRef);
    if (!container) {
      return;
    }

    const mermaid = await getMermaid();
    const mermaidBlocks = Array.from(
      container.querySelectorAll<HTMLElement>('code.language-mermaid')
    );

    for (const codeBlock of mermaidBlocks) {
      const preBlock = codeBlock.closest('pre');
      if (!preBlock || preBlock.dataset.mermaidProcessed === 'true') {
        continue;
      }

      preBlock.dataset.mermaidProcessed = 'true';

      const definition = codeBlock.textContent?.trim();
      if (!definition) {
        preBlock.classList.add('mermaid-error');
        if (!preBlock.querySelector('.mermaid-error-message')) {
          preBlock.insertAdjacentHTML('beforeend', renderErrorMessageHtml(new Error('Empty Mermaid definition.')));
        }
        continue;
      }

      try {
        const renderId = `mermaid-diagram-${mermaidRenderSequence++}`;
        const { svg } = await mermaid.render(renderId, definition);
        preBlock.outerHTML = `<div class="mermaid-rendered">${svg}</div>`;
      } catch (error) {
        preBlock.classList.add('mermaid-error');
        if (!preBlock.querySelector('.mermaid-error-message')) {
          preBlock.insertAdjacentHTML('beforeend', renderErrorMessageHtml(error));
        }
      }
    }
  }

  return {
    renderMermaidBlocks,
  };
}
