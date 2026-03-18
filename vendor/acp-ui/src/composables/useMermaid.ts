import type mermaidType from 'mermaid';

let mermaidPromise: Promise<typeof mermaidType> | null = null;
let mermaidRenderSequence = 0;

async function getMermaid(): Promise<typeof mermaidType> {
  if (!mermaidPromise) {
    mermaidPromise = import('mermaid').then((module) => {
      const mermaid = module.default;
      mermaid.initialize({
        startOnLoad: false,
        theme: 'dark',
      });
      return mermaid;
    });
  }

  return mermaidPromise;
}

function createErrorMessage(error: unknown): HTMLDivElement {
  const errorMessage = document.createElement('div');
  errorMessage.className = 'mermaid-error-message';
  errorMessage.textContent = `Unable to render Mermaid diagram: ${
    error instanceof Error ? error.message : 'Unknown error'
  }`;
  return errorMessage;
}

export function useMermaid() {
  async function renderMermaidBlocks(container: HTMLElement): Promise<void> {
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
          preBlock.append(createErrorMessage(new Error('Empty Mermaid definition.')));
        }
        continue;
      }

      try {
        const renderId = `mermaid-diagram-${mermaidRenderSequence++}`;
        const { svg } = await mermaid.render(renderId, definition);
        const renderedBlock = document.createElement('div');
        renderedBlock.className = 'mermaid-rendered';
        renderedBlock.innerHTML = svg;
        preBlock.replaceWith(renderedBlock);
      } catch (error) {
        preBlock.classList.add('mermaid-error');
        if (!preBlock.querySelector('.mermaid-error-message')) {
          preBlock.append(createErrorMessage(error));
        }
      }
    }
  }

  return {
    renderMermaidBlocks,
  };
}
