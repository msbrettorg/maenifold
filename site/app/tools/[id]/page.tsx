// T-SITE-001.9c: RTM FR-15.11, FR-15.21 — Tool detail page (static, data-driven)
import fs from 'fs';
import path from 'path';
import type { Metadata } from 'next';
import { renderMarkdown } from '@/lib/markdown';

// --- Data loading ---

const toolsDir = path.join(process.cwd(), '..', 'src', 'assets', 'usage', 'tools');

interface ToolData {
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

function loadTool(id: string): ToolData | null {
  const filePath = path.join(toolsDir, `${id}.md`);
  if (!fs.existsSync(filePath)) return null;
  const source = fs.readFileSync(filePath, 'utf-8');
  const { name, description } = parseToolFile(source);

  // Strip the first heading line so we don't render two <h1> tags
  const contentWithoutHeading = source
    .split('\n')
    .filter((line, index) => {
      // Remove the very first # heading line
      if (index === 0 && line.startsWith('# ')) return false;
      return true;
    })
    .join('\n')
    // Trim leading blank lines left after stripping the heading
    .replace(/^\s*\n/, '');

  return { name, description, content: contentWithoutHeading };
}

// --- Static params for `output: 'export'` ---

export function generateStaticParams(): { id: string }[] {
  const files = fs.readdirSync(toolsDir).filter((f) => f.endsWith('.md'));
  return files.map((f) => ({ id: f.replace('.md', '') }));
}

// --- Metadata ---

export async function generateMetadata({
  params,
}: {
  params: Promise<{ id: string }>;
}): Promise<Metadata> {
  const { id } = await params;
  const tool = loadTool(id);
  if (!tool) {
    return { title: 'Tool Not Found — maenifold' };
  }
  return {
    title: `${tool.name} — maenifold`,
    description: tool.description || `${tool.name} tool documentation for maenifold.`,
  };
}

// --- Page component ---

export default async function ToolDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const tool = loadTool(id);

  if (!tool) {
    return (
      <main className="prose-width" style={{ padding: '4rem 1.5rem' }}>
        <a href="/tools" style={{ fontSize: '0.875rem', color: 'var(--accent)' }}>
          &larr; Tools
        </a>
        <h1 style={{ marginTop: '2rem' }}>Tool not found</h1>
        <p style={{ color: 'var(--text-secondary)' }}>
          No tool with id &ldquo;{id}&rdquo; exists.
        </p>
      </main>
    );
  }

  const contentHtml = await renderMarkdown(tool.content);

  return (
    <main className="prose-width" style={{ padding: '4rem 1.5rem' }}>
      {/* Back link */}
      <nav style={{ marginBottom: '2rem' }}>
        <a href="/tools" style={{ fontSize: '0.875rem' }}>
          &larr; Tools
        </a>
      </nav>

      {/* Header */}
      <header style={{ marginBottom: '2rem' }}>
        <h1 style={{ margin: 0 }}>{tool.name}</h1>
        {tool.description && (
          <p style={{ margin: '0.5rem 0 0', color: 'var(--text-secondary)', lineHeight: 1.6 }}>
            {tool.description}
          </p>
        )}
      </header>

      {/* Full markdown content */}
      <div
        className="markdown-content"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: build-time markdown render
        // codeql[js/stored-xss] Source files are repo-committed .md files read at build time
        // (next build, output: 'export'). No runtime server. No user-controlled input reaches
        // this pipeline. allowDangerousHtml + rehypeRaw required for inline HTML in docs.
        dangerouslySetInnerHTML={{ __html: contentHtml }}
      />
    </main>
  );
}
