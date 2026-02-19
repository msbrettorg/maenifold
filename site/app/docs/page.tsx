// T-SITE-001.7: RTM FR-15.9 — /docs page rendering docs/README.md
// T-SITE-001.21: Sidebar layout, link transformation
import { readFileSync } from 'fs';
import { join } from 'path';
import { renderMarkdown, transformDocsLinks, stripTableOfContents } from '@/lib/markdown';

export const metadata = {
  title: 'Documentation — maenifold',
  description: 'Architecture, philosophy, and detailed guide for maenifold.',
};

export default async function DocsPage() {
  const raw = readFileSync(join(process.cwd(), '..', 'docs', 'README.md'), 'utf-8');
  const source = transformDocsLinks(stripTableOfContents(raw));
  const html = await renderMarkdown(source);
  return <div dangerouslySetInnerHTML={{ __html: html }} />;
}
