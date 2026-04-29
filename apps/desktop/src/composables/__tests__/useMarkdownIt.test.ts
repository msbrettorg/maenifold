import { beforeAll, describe, expect, it } from 'vitest';
import { getMarkdownIt } from '../useMarkdownIt';

describe('useMarkdownIt SVG passthrough regression', () => {
  let render: (source: string) => string;

  beforeAll(async () => {
    const md = await getMarkdownIt();
    render = (source: string) => md.render(source);
  });

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
});
