// T-SITE-001.7: RTM FR-15.9 — /docs page rendering docs/README.md
import { readFileSync } from 'fs';
import { join } from 'path';
import { renderMarkdown } from '@/lib/markdown';

export const metadata = {
  title: 'Documentation — maenifold',
  description: 'Architecture, philosophy, and detailed guide for maenifold.',
};

export default async function DocsPage() {
  const source = readFileSync(join(process.cwd(), '..', 'docs', 'README.md'), 'utf-8');
  const html = await renderMarkdown(source);
  return (
    <main className="prose-width markdown-content" style={{ padding: '3rem 1rem 5rem' }}>
      <div dangerouslySetInnerHTML={{ __html: html }} />
    </main>
  );
}
