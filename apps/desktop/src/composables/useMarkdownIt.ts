import MarkdownIt from 'markdown-it';
import type { StateInline } from 'markdown-it/index.js';
import Shiki from '@shikijs/markdown-it';

let mdPromise: Promise<MarkdownIt> | null = null;

/**
 * Inline rule that recognises [[concept]] WikiLinks and emits a 'wikilink'
 * token. Keeping this at the parser level lets us leave `html: false` on the
 * markdown-it instance (no raw HTML pass-through, no XSS surface), while still
 * rendering pill-styled spans for WikiLinks.
 */
function wikilinkPlugin(md: MarkdownIt): void {
  md.inline.ruler.before('emphasis', 'wikilink', (state: StateInline, silent: boolean) => {
    const start = state.pos;
    const src = state.src;
    if (src.charCodeAt(start) !== 0x5b /* [ */ || src.charCodeAt(start + 1) !== 0x5b) {
      return false;
    }
    const closeIdx = src.indexOf(']]', start + 2);
    if (closeIdx < 0) return false;

    const label = src.slice(start + 2, closeIdx);
    // Disallow newlines and nested brackets inside the label.
    if (!label || /[\r\n\[\]]/.test(label)) return false;

    if (!silent) {
      const token = state.push('wikilink', '', 0);
      token.content = label;
      token.markup = '[[]]';
    }
    state.pos = closeIdx + 2;
    return true;
  });

  md.renderer.rules.wikilink = (tokens, idx) => {
    const label = md.utils.escapeHtml(tokens[idx].content);
    return `<span class="note-wiki-pill">${label}</span>`;
  };
}

/**
 * Lazily initialize a single markdown-it instance configured with:
 *  - Shiki syntax highlighting (light + dark themes via CSS variables).
 *  - A WikiLink inline rule that emits styled pill spans for [[concept]].
 *  - A fence override that leaves ```mermaid blocks untouched so the
 *    useMermaid composable can post-process them into rendered SVG.
 *
 * `html: false` is intentional — raw HTML in source must never bypass the
 * tokeniser. WikiLink pills are emitted by a renderer rule whose output is
 * still routed through DOMPurify by the consumer.
 *
 * Returns a shared promise so the (async) Shiki plugin is only loaded once.
 */
export function getMarkdownIt(): Promise<MarkdownIt> {
  if (!mdPromise) {
    mdPromise = (async () => {
      const md = new MarkdownIt({
        html: false,
        linkify: true,
        breaks: false,
        typographer: false,
      });

      md.use(
        await Shiki({
          themes: {
            light: 'github-light',
            dark: 'github-dark',
          },
        }),
      );

      md.use(wikilinkPlugin);

      // Preserve mermaid fences as plain <pre><code class="language-mermaid">
      // so useMermaid().renderMermaidBlocks() can find and replace them.
      const defaultFence = md.renderer.rules.fence;
      md.renderer.rules.fence = (tokens, idx, options, env, self) => {
        const token = tokens[idx];
        const info = (token.info ?? '').trim().toLowerCase();
        if (info === 'mermaid') {
          const escaped = md.utils.escapeHtml(token.content);
          return `<pre class="language-mermaid"><code class="language-mermaid">${escaped}</code></pre>\n`;
        }
        return defaultFence
          ? defaultFence(tokens, idx, options, env, self)
          : self.renderToken(tokens, idx, options);
      };

      return md;
    })();
  }
  return mdPromise;
}

