import { beforeAll, describe, expect, it } from 'vitest';
import DOMPurify from 'dompurify';
import { getMarkdownIt } from '../useMarkdownIt';

describe('useMarkdownIt SVG passthrough regression', () => {
  let render: (source: string) => string;
  const allowedUriRegexp =
    /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$)|data:image\/(?:bmp|gif|jpe?g|png|webp);base64,)/i;
  const forbiddenTags = ['script', 'iframe', 'object', 'embed', 'form', 'input', 'button', 'textarea'];
  const eventAttributeHookInstalledKey = '__maenifoldEventAttributeHookInstalled';

  beforeAll(async () => {
    const md = await getMarkdownIt();
    render = (source: string) => md.render(source);
  });

  function installEventAttributeHook(): void {
    const purifierState = DOMPurify as unknown as Record<string, boolean>;
    if (purifierState[eventAttributeHookInstalledKey]) return;
    DOMPurify.addHook('uponSanitizeAttribute', (_node, data) => {
      if (data.attrName?.toLowerCase().startsWith('on')) {
        data.keepAttr = false;
      }
    });
    purifierState[eventAttributeHookInstalledKey] = true;
  }

  function sanitizeLikeMarkdownView(html: string): string {
    installEventAttributeHook();
    return DOMPurify.sanitize(html, {
      USE_PROFILES: { html: true, svg: true, svgFilters: true },
      ADD_TAGS: ['style', 'foreignObject', 'use'],
      ADD_ATTR: [
        'class',
        'style',
        'data-wikilink',
        'data-mermaid-processed',
        'disabled',
        'checked',
        'xmlns',
        'viewBox',
        'fill',
        'stroke',
        'd',
        'cx',
        'cy',
        'r',
        'width',
        'height',
        'x',
        'y',
        'rx',
        'ry',
        'transform',
        'points',
        'x1',
        'y1',
        'x2',
        'y2',
        'id',
        'href',
        'xlink:href',
        'xmlns:xlink',
      ],
      FORBID_TAGS: forbiddenTags,
      ALLOWED_URI_REGEXP: allowedUriRegexp,
    });
  }

  function renderSanitized(source: string): string {
    return sanitizeLikeMarkdownView(render(source));
  }

  function textContent(html: string): string {
    return new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html').body.textContent ?? '';
  }

  it('raw <svg> in markdown renders visible shape', () => {
    const html = render('<svg xmlns="http://www.w3.org/2000/svg"><circle cx="5" cy="5" r="4" /></svg>');

    expect(html).toContain('<svg');
    expect(html).toContain('<circle');
  });

  it('<svg> with viewBox attribute preserved', () => {
    const html = render('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0 L10 10" /></svg>');

    expect(html).toContain('<svg');
    expect(html).toContain('viewBox="0 0 10 10"');
    expect(html).toContain('<path');
  });

  it('empty <svg></svg> does not crash', () => {
    const html = render('<svg xmlns="http://www.w3.org/2000/svg"></svg>');

    expect(html).toContain('<svg');
    expect(html).not.toContain('&lt;svg');
  });

  it('<svg> with class and style attributes preserved', () => {
    const html = render('<svg xmlns="http://www.w3.org/2000/svg" class="diagram" style="color: red"><rect width="10" height="10" /></svg>');

    expect(html).toContain('<svg');
    expect(html).toContain('class="diagram"');
    expect(html).toContain('style="color: red"');
    expect(html).toContain('<rect');
  });

  describe('KaTeX inline math rendering', () => {
    it('renders $x^2$ with KaTeX markup', () => {
      const html = render('$x^2$');

      expect(html).toContain('<span class="katex"');
      expect(html).not.toContain('<p>$x^2$</p>');
    });

    it('renders Greek letters for $\\alpha + \\beta$', () => {
      const html = render('$\\alpha + \\beta$');

      expect(html).toContain('<span class="katex"');
      expect(textContent(html)).toMatch(/α.*β/s);
    });

    it('renders subscripts for $x_1 + x_2$', () => {
      const html = render('$x_1 + x_2$');

      expect(html).toContain('<span class="katex"');
      expect(html).toContain('<msub>');
    });

    it('keeps $$$3$ literal instead of treating it as math', () => {
      const html = render('$$$3$');

      expect(html).toContain('$$$3$');
      expect(html).not.toContain('<span class="katex"');
    });

    it('renders inline math embedded in other markdown', () => {
      const html = render('Inline $x$ here.');

      expect(html).toContain('Inline ');
      expect(html).toContain('<span class="katex"');
      expect(html).toContain(' here.');
    });
  });

  describe('KaTeX block math rendering', () => {
    it('renders $$x^2 + y^2 = z^2$$ as display math', () => {
      const html = render('$$x^2 + y^2 = z^2$$');

      expect(html).toContain('<span class="katex-display"');
      expect(html).not.toContain('<p>$$x^2 + y^2 = z^2$$</p>');
    });

    it('renders multi-line block math', () => {
      const html = render('$$\n\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}\n$$');

      expect(html).toContain('<span class="katex-display"');
      expect(textContent(html)).toContain('n(n+1)');
    });

    it('renders block math with surrounding paragraphs', () => {
      const html = render('Before.\n\n$$x^2 + y^2 = z^2$$\n\nAfter.');

      expect(html).toContain('<p>Before.</p>');
      expect(html).toContain('<span class="katex-display"');
      expect(html).toContain('<p>After.</p>');
    });
  });

  describe('KaTeX edge cases', () => {
    it('does not crash on empty inline $$', () => {
      expect(() => render('$$')).not.toThrow();
    });

    it('does not crash on empty block $$\\n\\n$$', () => {
      expect(() => render('$$\n\n$$')).not.toThrow();
    });

    it('renders invalid math with throwOnError disabled', () => {
      const html = render('$\\notacommand$');

      expect(html).toContain('katex-error');
      expect(textContent(html)).toContain('\\notacommand');
    });

    it('keeps currency-style unmatched dollars as plain text', () => {
      const html = render('$5 and $10');

      expect(html).toContain('$5 and $10');
      expect(html).not.toContain('<span class="katex"');
    });

    it('sanitizes HTML-like math payloads without introducing XSS', () => {
      const html = renderSanitized('$<script>$');

      expect(html).toContain('<span class="katex"');
      expect(html).not.toContain('<script');
      expect(html).not.toContain('onerror=');
    });
  });

  describe('KaTeX sanitizer interaction', () => {
    it('preserves KaTeX output through MarkdownView DOMPurify sanitization', () => {
      const html = renderSanitized('$x^2$');

      expect(html).toContain('<span class="katex"');
      expect(html).not.toContain('&lt;span class="katex"');
    });

    it('keeps KaTeX required attributes after uponSanitizeAttribute hardening', () => {
      const html = renderSanitized('$x^2$');

      expect(html).toContain('class="katex"');
      expect(html).toContain('aria-hidden="true"');
      expect(html).not.toContain('onclick=');
    });
  });
});
