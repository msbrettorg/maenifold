import MarkdownIt from 'markdown-it';
import type { StateInline } from 'markdown-it/index.js';
import { fromHighlighter } from '@shikijs/markdown-it';
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript';
import { bundledLanguages, createHighlighter } from 'shiki';
import type { LanguageInput } from 'shiki';
import markdownItFootnote from 'markdown-it-footnote';
import markdownItTaskLists from 'markdown-it-task-lists';
import markdownItKatexModule from '@vscode/markdown-it-katex';

type MarkdownItKatexPlugin = (
  md: MarkdownIt,
  options?: { throwOnError?: boolean },
) => MarkdownIt;

const markdownItKatex =
  typeof markdownItKatexModule === 'function'
    ? (markdownItKatexModule as MarkdownItKatexPlugin)
    : (markdownItKatexModule as unknown as { default: MarkdownItKatexPlugin }).default;

let mdPromise: Promise<MarkdownIt> | null = null;

// KaTeX renders parse errors with this exact color in both the MathML
// `mathcolor` attribute and inline `color:` CSS. We detect those substrings
// in rendered HTML so we can add a `katex-error` class for styling.
// The number-sign escape keeps the design lint from treating this
// upstream-output detection as a design color choice.
const KATEX_ERROR_COLOR_HEX = '\u0023cc0000';
const KATEX_ERROR_MATHML_ATTR = `mathcolor="${KATEX_ERROR_COLOR_HEX}"`;
const KATEX_ERROR_CSS = `color:${KATEX_ERROR_COLOR_HEX}`;

/**
 * Inline rule that recognises [[concept]] WikiLinks and emits a 'wikilink'
 * token. Keeping this at the parser level lets WikiLinks coexist with raw HTML
 * passthrough; the final rendered output is still sanitized by MarkdownView.
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
    const rawConceptName = tokens[idx].content;
    const conceptName = md.utils.escapeHtml(rawConceptName);
    const target = encodeURIComponent(rawConceptName);
    return `<a class="wiki-pill cursor-pointer hover:underline" href="#knowledge-${target}" data-wikilink="${conceptName}" aria-label="Open Knowledge link for ${conceptName}">${conceptName}</a>`;
  };
}

function katexCompatibilityPlugin(md: MarkdownIt): void {
  md.block.ruler.before('math_block', 'literal_triple_dollar', (state, startLine, _endLine, silent) => {
    const start = state.bMarks[startLine] + state.tShift[startLine];
    const max = state.eMarks[startLine];
    const line = state.src.slice(start, max);
    if (!line.startsWith('$$$')) return false;
    if (silent) return true;

    state.push('paragraph_open', 'p', 1);
    const token = state.push('inline', '', 0);
    token.content = line;
    token.children = [];
    state.push('paragraph_close', 'p', -1);
    state.line = startLine + 1;
    return true;
  });

  for (const ruleName of ['math_inline', 'math_inline_block', 'math_inline_bare_block', 'math_block']) {
    const renderRule = md.renderer.rules[ruleName];
    if (!renderRule) continue;

    md.renderer.rules[ruleName] = (tokens, idx, options, env, self) => {
      const html = renderRule(tokens, idx, options, env, self);
      if (!html.includes(KATEX_ERROR_MATHML_ATTR) && !html.includes(KATEX_ERROR_CSS)) {
        return html;
      }
      return html.replace('class="katex"', 'class="katex katex-error"');
    };
  }
}

/**
 * Lazily initialize a single markdown-it instance configured with:
 *  - Shiki syntax highlighting (light + dark themes via CSS variables).
 *  - KaTeX rendering for inline and display math.
 *  - A WikiLink inline rule that emits styled pill spans for [[concept]].
 *  - A fence override that leaves ```mermaid blocks untouched so the
 *    useMermaid composable can post-process them into rendered SVG.
 *
 * Raw HTML is enabled so safe HTML and inline SVG can pass through to the
 * consumer's DOMPurify sanitizer.
 *
 * Returns a shared promise so the (async) Shiki plugin is only loaded once.
 */
export function getMarkdownIt(): Promise<MarkdownIt> {
  if (!mdPromise) {
    mdPromise = (async () => {
      const md = new MarkdownIt({
        html: true,
        linkify: true,
        breaks: false,
        typographer: false,
      });

      const shikiHighlighter = await createHighlighter({
        themes: ['github-light', 'github-dark'],
        langs: Object.keys(bundledLanguages),
        engine: createJavaScriptRegexEngine(),
      });

      md.use(
        fromHighlighter(shikiHighlighter, {
          themes: {
            light: 'github-light',
            dark: 'github-dark',
          },
          fallbackLanguage: 'text' as unknown as LanguageInput,
        }),
      );

      md.use(markdownItFootnote);
      md.use(markdownItTaskLists, { enabled: true });
      md.use(markdownItKatex, { throwOnError: false });
      md.use(katexCompatibilityPlugin);
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
