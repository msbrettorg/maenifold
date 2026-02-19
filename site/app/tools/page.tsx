// T-SITE-001.9: RTM FR-15.11, FR-15.21, FR-15.22 — /tools data-driven catalog from src/assets/usage/tools/
import { readdirSync, readFileSync } from 'fs';
import { join } from 'path';
import { renderMarkdown } from '@/lib/markdown';

export const metadata = {
  title: 'Tools — maenifold',
  description: 'Complete reference for all maenifold tools.',
};

interface ToolEntry {
  slug: string;
  name: string;
  description: string;
  content: string;
}

function parseToolFile(source: string): { name: string; description: string } {
  const lines = source.split('\n');

  // Extract name from first # heading
  const headingLine = lines.find((line) => line.startsWith('# '));
  const name = headingLine ? headingLine.replace(/^# /, '').trim() : 'Unknown';

  // Extract first non-empty paragraph after the heading
  let foundHeading = false;
  let description = '';
  for (const line of lines) {
    if (!foundHeading) {
      if (line.startsWith('# ')) {
        foundHeading = true;
      }
      continue;
    }
    // Skip blank lines immediately after heading
    if (!description && line.trim() === '') continue;
    // Stop at next heading or blank line after we have content
    if (description && line.trim() === '') break;
    if (line.startsWith('#')) break;
    description += (description ? ' ' : '') + line.trim();
  }

  return { name, description };
}

function slugify(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, '');
}

export default async function ToolsPage() {
  const toolsDir = join(process.cwd(), '..', 'src', 'assets', 'usage', 'tools');
  const files = readdirSync(toolsDir)
    .filter((f) => f.endsWith('.md'))
    .sort();

  const tools: ToolEntry[] = files.map((filename) => {
    const source = readFileSync(join(toolsDir, filename), 'utf-8');
    const { name, description } = parseToolFile(source);
    return {
      slug: slugify(name),
      name,
      description,
      content: source,
    };
  });

  // Render all tool content at build time
  const renderedTools = await Promise.all(
    tools.map(async (tool) => ({
      ...tool,
      html: await renderMarkdown(tool.content),
    }))
  );

  const toolCount = tools.length;

  return (
    <main className="prose-width" style={{ padding: '3rem 1rem 5rem' }}>
      <h1 style={{ marginBottom: '0.5rem' }}>Tools</h1>
      <p className="text-text-secondary" style={{ marginTop: 0, marginBottom: '2.5rem' }}>
        {toolCount} tools available
      </p>

      {/* Table of contents */}
      <nav aria-label="Tools" style={{ marginBottom: '3rem' }}>
        <h2 style={{ fontSize: '16px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-secondary)', marginBottom: '0.75rem', borderBottom: 'none' }}>
          Contents
        </h2>
        <ol style={{ columns: 2, columnGap: '2rem', padding: 0, margin: 0, listStyle: 'none' }}>
          {renderedTools.map((tool) => (
            <li key={tool.slug} style={{ marginBottom: '0.25rem' }}>
              <a href={`#${tool.slug}`}>{tool.name}</a>
            </li>
          ))}
        </ol>
      </nav>

      {/* Tool entries */}
      {renderedTools.map((tool) => (
        <section key={tool.slug} id={tool.slug} style={{ marginBottom: '4rem' }}>
          <h2 style={{ marginBottom: '0.5rem' }}>{tool.name}</h2>
          <p style={{ color: 'var(--text-secondary)', marginTop: 0, marginBottom: '1.5rem' }}>
            {tool.description}
          </p>
          <div className="markdown-content" dangerouslySetInnerHTML={{ __html: tool.html }} />
        </section>
      ))}
    </main>
  );
}
