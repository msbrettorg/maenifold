// T-SITE-001.9b: RTM FR-15.11, FR-15.21, FR-15.22 — /tools card grid catalog from src/assets/usage/tools/
import Link from 'next/link';
import { readdirSync, readFileSync } from 'fs';
import { join } from 'path';

export const metadata = {
  title: 'Tools — maenifold',
  description: 'Complete reference for all maenifold tools.',
};

interface ToolEntry {
  slug: string;
  name: string;
  description: string;
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

export default function ToolsPage() {
  const toolsDir = join(process.cwd(), '..', 'src', 'assets', 'usage', 'tools');
  const files = readdirSync(toolsDir).filter((f) => f.endsWith('.md'));

  const tools: ToolEntry[] = files
    .map((filename) => {
      const source = readFileSync(join(toolsDir, filename), 'utf-8');
      const { name, description } = parseToolFile(source);
      const slug = filename.replace('.md', '');
      return { slug, name, description };
    })
    .sort((a, b) => a.name.localeCompare(b.name));

  const count = tools.length;

  return (
    <main style={{ maxWidth: '1100px', marginInline: 'auto', padding: '4rem 1.5rem' }}>
      <style>{`
        .workflow-card {
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 8px;
          padding: 1.5rem;
          transition: border-color 0.15s ease;
          cursor: pointer;
        }
        .workflow-card:hover {
          border-color: var(--accent-muted);
        }
        .workflow-card-link {
          text-decoration: none;
          color: inherit;
          display: block;
        }
      `}</style>

      <header style={{ marginBottom: '2.5rem' }}>
        <h1 style={{ marginBottom: '0.5rem' }}>Tools</h1>
        <p style={{ color: 'var(--text-secondary)', margin: 0 }}>
          {count} tools available
        </p>
      </header>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
          gap: '1rem',
        }}
      >
        {tools.map((tool) => (
          <Link
            key={tool.slug}
            href={`/tools/${tool.slug}`}
            className="workflow-card-link"
          >
            <article className="workflow-card">
              <h2 style={{ margin: '0 0 0.625rem', fontSize: '1rem', fontWeight: 600 }}>
                {tool.name}
              </h2>
              {tool.description && (
                <p
                  style={{
                    margin: 0,
                    fontSize: '0.875rem',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                  }}
                >
                  {tool.description}
                </p>
              )}
            </article>
          </Link>
        ))}
      </div>
    </main>
  );
}
