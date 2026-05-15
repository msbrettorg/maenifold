import type mermaidType from 'mermaid';
import type { Ref } from 'vue';
import { sanitize } from '../lib/sanitizer';

let mermaidPromise: Promise<typeof mermaidType> | null = null;
let mermaidRenderSequence = 0;

function getMermaidTheme(): 'dark' | 'default' {
  const isDark =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
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

function sanitizeMermaidSvg(svg: string): string {
  return sanitize(svg, {
    USE_PROFILES: { svg: true, svgFilters: true },
    ADD_TAGS: ['style', 'foreignObject'],
    ADD_ATTR: ['xmlns'],
  });
}

function appendMermaidErrorMessage(preBlock: Element, error: unknown): void {
  if (preBlock.querySelector('.mermaid-error-message')) {
    return;
  }

  const message = error instanceof Error ? error.message : 'Unknown error';
  const errorElement = document.createElement('div');
  errorElement.className = 'mermaid-error-message';
  errorElement.textContent = `Unable to render Mermaid diagram: ${message}`;
  preBlock.append(errorElement);
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

    const mermaidBlocks = Array.from(
      container.querySelectorAll<HTMLElement>('code.language-mermaid')
    );
    if (mermaidBlocks.length === 0) {
      return;
    }

    const mermaid = await getMermaid();

    for (const codeBlock of mermaidBlocks) {
      const preBlock = codeBlock.closest('pre');
      if (!preBlock || preBlock.dataset.mermaidProcessed === 'true') {
        continue;
      }

      preBlock.dataset.mermaidProcessed = 'true';

      const definition = codeBlock.textContent?.trim();
      if (!definition) {
        preBlock.classList.add('mermaid-error');
        appendMermaidErrorMessage(preBlock, new Error('Empty Mermaid definition.'));
        continue;
      }

      try {
        const renderId = `mermaid-diagram-${mermaidRenderSequence++}`;
        const { svg } = await mermaid.render(renderId, definition);
        const sanitizedSvg = sanitizeMermaidSvg(svg);
        const renderedContainer = document.createElement('div');
        renderedContainer.className = 'mermaid-rendered';
        renderedContainer.innerHTML = sanitizedSvg;
        preBlock.replaceWith(renderedContainer);
      } catch (error) {
        preBlock.classList.add('mermaid-error');
        appendMermaidErrorMessage(preBlock, error);
      }
    }
  }

  return {
    renderMermaidBlocks,
  };
}
