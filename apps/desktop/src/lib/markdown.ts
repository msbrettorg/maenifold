import { marked } from 'marked';
import { markedHighlight } from 'marked-highlight';
import hljs from 'highlight.js/lib/common';

let configured = false;

/**
 * Configure the singleton `marked` instance with syntax highlighting via
 * highlight.js. Safe to call more than once — only the first call applies.
 *
 * Fenced code blocks (```lang ... ```) are rendered as
 * `<pre><code class="hljs language-<lang>">…tokens…</code></pre>` so the
 * highlight.js theme CSS imported in `main.ts` colors them.
 */
export function configureMarkdown(): void {
  if (configured) return;
  configured = true;

  marked.use(
    markedHighlight({
      emptyLangClass: 'hljs',
      langPrefix: 'hljs language-',
      highlight(code, lang) {
        const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
        try {
          return hljs.highlight(code, { language, ignoreIllegals: true }).value;
        } catch {
          return code;
        }
      },
    }),
  );
}
