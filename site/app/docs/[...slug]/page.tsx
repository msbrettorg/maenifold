// T-SITE-001.21: Catch-all route for docs sub-pages
// Serves any markdown file from docs/ directory
import { readFileSync, readdirSync, statSync, existsSync } from 'fs';
import { join } from 'path';
import { notFound } from 'next/navigation';
import { renderMarkdown, transformDocsLinks } from '@/lib/markdown';

const DOCS_ROOT = join(process.cwd(), '..', 'docs');

/** Recursively list all .md files relative to a root directory. */
function listMarkdownFiles(dir: string, prefix = ''): string[] {
  const results: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const rel = prefix ? `${prefix}/${entry}` : entry;
    if (statSync(full).isDirectory()) {
      results.push(...listMarkdownFiles(full, rel));
    } else if (entry.endsWith('.md') && entry !== 'README.md') {
      // Strip .md → becomes the URL slug
      results.push(rel.replace(/\.md$/, ''));
    }
  }
  return results;
}

export function generateStaticParams() {
  return listMarkdownFiles(DOCS_ROOT).map(path => ({
    slug: path.split('/'),
  }));
}

export async function generateMetadata({ params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = await params;
  const title = slug[slug.length - 1]
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());
  return {
    title: `${title} — maenifold docs`,
  };
}

export default async function DocsSubPage({ params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = await params;
  const filePath = join(DOCS_ROOT, slug.join('/') + '.md');

  if (!existsSync(filePath)) {
    notFound();
  }

  const raw = readFileSync(filePath, 'utf-8');
  const source = transformDocsLinks(raw);
  const html = await renderMarkdown(source);

  return (
    <div>
      <a
        href="/docs"
        style={{
          display: 'inline-block',
          fontSize: '0.875rem',
          color: 'var(--text-secondary)',
          marginBottom: '2rem',
        }}
      >
        &larr; Documentation
      </a>
      <div dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}
