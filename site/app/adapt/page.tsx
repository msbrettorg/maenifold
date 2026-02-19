// T-SITE-001.20: RTM FR-15.21 — /adapt catalog page for roles, colors, and perspectives
import Link from 'next/link';
import fs from 'fs';
import path from 'path';

export const metadata = {
  title: 'Adapt — maenifold',
  description:
    'Test-time adaptations: roles, colors, and perspectives for conditioning agent reasoning.',
};

interface AdaptAsset {
  id: string;
  name: string;
  emoji: string;
  description: string;
}

function loadAssets(type: string): AdaptAsset[] {
  const dir = path.join(process.cwd(), '..', 'src', 'assets', type);
  const files = fs.readdirSync(dir).filter((f) => f.endsWith('.json'));
  return files
    .map((f) => {
      const data = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf-8')) as AdaptAsset;
      return {
        id: data.id,
        name: data.name,
        emoji: data.emoji,
        description: data.description,
      };
    })
    .sort((a, b) => a.name.localeCompare(b.name));
}

interface SectionProps {
  title: string;
  type: string;
  items: AdaptAsset[];
}

function AdaptSection({ title, type, items }: SectionProps) {
  return (
    <section style={{ marginBottom: '3.5rem' }}>
      <h2 style={{ marginBottom: '0.25rem', fontSize: '1.25rem', fontWeight: 600 }}>
        {title}
      </h2>
      <p style={{ color: 'var(--text-secondary)', margin: '0 0 1.25rem', fontSize: '0.9rem' }}>
        {items.length} {title.toLowerCase()}
      </p>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
          gap: '1rem',
        }}
      >
        {items.map((item) => (
          <Link
            key={item.id}
            href={`/adapt/${type}/${item.id}`}
            className="workflow-card-link"
          >
            <article className="workflow-card">
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.625rem',
                  marginBottom: '0.625rem',
                }}
              >
                <span style={{ fontSize: '1.25rem', lineHeight: 1 }} aria-hidden="true">
                  {item.emoji}
                </span>
                <h3 style={{ margin: 0, fontSize: '1rem', fontWeight: 600 }}>
                  {item.name}
                </h3>
              </div>
              {item.description && (
                <p
                  style={{
                    margin: 0,
                    fontSize: '0.875rem',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                  }}
                >
                  {item.description}
                </p>
              )}
            </article>
          </Link>
        ))}
      </div>
    </section>
  );
}

export default function AdaptPage() {
  const roles = loadAssets('roles');
  const colors = loadAssets('colors');
  const perspectives = loadAssets('perspectives');
  const total = roles.length + colors.length + perspectives.length;

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
        <h1 style={{ marginBottom: '0.5rem' }}>Adapt</h1>
        <p style={{ color: 'var(--text-secondary)', margin: '0 0 1rem', maxWidth: '640px', lineHeight: 1.7 }}>
          The{' '}
          <a href="/workflows/agentic-research" style={{ color: 'var(--accent)' }}>research</a>
          {' '}and{' '}
          <a href="/workflows/think-tank" style={{ color: 'var(--accent)' }}>think tank</a>
          {' '}workflows seed the knowledge graph. The{' '}
          <a href="/workflows/role-creation-workflow" style={{ color: 'var(--accent)' }}>role creation</a>
          {' '}workflow reads the graph to produce new roles. Roles and the graph combine to
          create new workflows. Every time you run a workflow or use the primitives, you feed
          the graph. The{' '}
          <a href="/workflows/memory-cycle" style={{ color: 'var(--accent)' }}>decay system</a>
          {' '}and memory maintenance workflows keep the signal-to-noise ratio high.
        </p>
        <p style={{ color: 'var(--text-secondary)', margin: '0 0 1rem', maxWidth: '640px', lineHeight: 1.7 }}>
          It is a factory for building domain experts and reasoning strategies from a
          graph of thought — and the graph gets better with every use. The{' '}
          <code
            style={{
              fontFamily: 'ui-monospace, monospace',
              fontSize: '0.875em',
              background: 'var(--bg-code)',
              padding: '0.1em 0.35em',
              borderRadius: '3px',
            }}
          >
            adopt
          </code>{' '}
          tool loads any of these at inference time.
        </p>
        <p style={{ color: 'var(--text-secondary)', margin: 0, fontSize: '0.9rem' }}>
          {total} adaptations available
        </p>
      </header>

      <AdaptSection title="Roles" type="roles" items={roles} />
      <AdaptSection title="Colors" type="colors" items={colors} />
      <AdaptSection title="Perspectives" type="perspectives" items={perspectives} />
    </main>
  );
}
