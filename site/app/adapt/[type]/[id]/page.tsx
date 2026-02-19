// T-SITE-001.21: RTM FR-15.21 — Adapt detail page (roles, colors, perspectives)
import fs from 'fs';
import path from 'path';
import type { Metadata } from 'next';

// --- Types ---

interface Personality {
  motto?: string;
  principles?: string[];
  communicationStyle?: string;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ApproachValue = string[] | Record<string, any> | string;

interface AdaptAsset {
  id: string;
  name: string;
  emoji: string;
  description?: string;
  triggers?: string[];
  personality?: Personality;
  approach?: Record<string, ApproachValue>;
  antiPatterns?: string[];
  instruction?: string;
}

type AssetType = 'roles' | 'colors' | 'perspectives';

// --- Helpers ---

function typeBadgeLabel(type: AssetType): string {
  if (type === 'roles') return 'Role';
  if (type === 'colors') return 'Color';
  return 'Perspective';
}

/** Convert camelCase or PascalCase to Title Case */
function toTitleCase(key: string): string {
  return key
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, (s) => s.toUpperCase())
    .trim();
}

/** Check whether a value is a plain object (not array, not null) */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function isPlainObject(value: unknown): value is Record<string, any> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

// --- Data loading ---

function assetsDir(type: AssetType): string {
  return path.join(process.cwd(), '..', 'src', 'assets', type);
}

function loadAsset(type: AssetType, id: string): AdaptAsset | null {
  const filePath = path.join(assetsDir(type), `${id}.json`);
  if (!fs.existsSync(filePath)) return null;
  return JSON.parse(fs.readFileSync(filePath, 'utf-8')) as AdaptAsset;
}

// --- Static params ---

export function generateStaticParams(): { type: string; id: string }[] {
  const types: AssetType[] = ['roles', 'colors', 'perspectives'];
  const params: { type: string; id: string }[] = [];
  for (const type of types) {
    const dir = assetsDir(type);
    if (!fs.existsSync(dir)) continue;
    const files = fs.readdirSync(dir).filter((f) => f.endsWith('.json'));
    for (const file of files) {
      params.push({ type, id: file.replace('.json', '') });
    }
  }
  return params;
}

// --- Metadata ---

export async function generateMetadata({
  params,
}: {
  params: Promise<{ type: string; id: string }>;
}): Promise<Metadata> {
  const { type, id } = await params;
  const asset = loadAsset(type as AssetType, id);
  if (!asset) {
    return { title: 'Not Found — maenifold' };
  }
  const label = typeBadgeLabel(type as AssetType);
  return {
    title: `${asset.name} — maenifold`,
    description: asset.description ?? `${asset.name} ${label.toLowerCase()} for maenifold.`,
  };
}

// --- Approach renderer ---

interface ApproachSectionProps {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  approach: Record<string, any>;
}

function ApproachSection({ approach }: ApproachSectionProps) {
  const entries = Object.entries(approach).filter(
    ([, value]) => value !== null && value !== undefined && typeof value !== 'function'
  );

  if (entries.length === 0) return null;

  return (
    <section style={{ marginBottom: '2.5rem' }}>
      <h2
        style={{
          fontSize: '0.8125rem',
          fontWeight: 600,
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          color: 'var(--text-secondary)',
          marginBottom: '1rem',
        }}
      >
        Approach
      </h2>
      {entries.map(([key, value]) => (
        <div key={key} style={{ marginBottom: '1.25rem' }}>
          <h3
            style={{
              fontSize: '0.9375rem',
              fontWeight: 600,
              margin: '0 0 0.5rem',
            }}
          >
            {toTitleCase(key)}
          </h3>
          <ApproachValue value={value} />
        </div>
      ))}
    </section>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function ApproachValue({ value }: { value: any }) {
  if (Array.isArray(value)) {
    return (
      <ul
        style={{
          margin: 0,
          paddingLeft: '1.25rem',
          display: 'flex',
          flexDirection: 'column',
          gap: '0.25rem',
        }}
      >
        {(value as string[]).map((item, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: static list, order is stable
          <li key={i} style={{ lineHeight: 1.65, fontSize: '0.9375rem' }}>
            {item}
          </li>
        ))}
      </ul>
    );
  }

  if (isPlainObject(value)) {
    return (
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '0.875rem',
          paddingLeft: '0.75rem',
          borderLeft: '2px solid var(--border)',
        }}
      >
        {Object.entries(value).map(([subKey, subValue]) => (
          <div key={subKey}>
            <div
              style={{
                fontSize: '0.8125rem',
                fontWeight: 600,
                color: 'var(--text-secondary)',
                marginBottom: '0.25rem',
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
              }}
            >
              {toTitleCase(subKey)}
            </div>
            {Array.isArray(subValue) ? (
              <ul
                style={{
                  margin: 0,
                  paddingLeft: '1.25rem',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '0.2rem',
                }}
              >
                {(subValue as string[]).map((item, i) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: static list, order is stable
                  <li key={i} style={{ lineHeight: 1.6, fontSize: '0.9rem' }}>
                    {item}
                  </li>
                ))}
              </ul>
            ) : (
              <ApproachValue value={subValue} />
            )}
          </div>
        ))}
      </div>
    );
  }

  if (typeof value === 'string') {
    return <p style={{ margin: 0, lineHeight: 1.65, fontSize: '0.9375rem' }}>{value}</p>;
  }

  return null;
}

// --- Page component ---

export default async function AdaptDetailPage({
  params,
}: {
  params: Promise<{ type: string; id: string }>;
}) {
  const { type, id } = await params;
  const assetType = type as AssetType;
  const asset = loadAsset(assetType, id);
  const label = typeBadgeLabel(assetType);

  if (!asset) {
    return (
      <main style={{ maxWidth: '72ch', marginInline: 'auto', padding: '4rem 1.5rem' }}>
        <a href="/adapt" style={{ fontSize: '0.875rem', color: 'var(--accent)' }}>
          &larr; Adapt
        </a>
        <h1 style={{ marginTop: '2rem' }}>Asset not found</h1>
        <p style={{ color: 'var(--text-secondary)' }}>
          No {label.toLowerCase()} with id &ldquo;{id}&rdquo; exists.
        </p>
      </main>
    );
  }

  return (
    <main style={{ maxWidth: '72ch', marginInline: 'auto', padding: '4rem 1.5rem' }}>
      <style>{`
        .trigger-chip {
          display: inline-block;
          font-family: ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace;
          font-size: 0.75rem;
          padding: 0.2em 0.55em;
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 4px;
          line-height: 1.5;
          white-space: nowrap;
        }
        .type-badge {
          display: inline-block;
          font-family: ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace;
          font-size: 0.7rem;
          padding: 0.2em 0.55em;
          background: var(--accent-muted, var(--bg-surface));
          color: var(--accent);
          border: 1px solid var(--border);
          border-radius: 4px;
          line-height: 1.5;
          white-space: nowrap;
          vertical-align: middle;
        }
        .anti-pattern-item {
          padding: 0.375rem 0.625rem;
          background: var(--bg-surface);
          border-left: 3px solid var(--border);
          border-radius: 0 4px 4px 0;
          font-size: 0.9rem;
          line-height: 1.6;
        }
      `}</style>

      {/* Back link */}
      <nav style={{ marginBottom: '2rem' }}>
        <a href="/adapt" style={{ fontSize: '0.875rem' }}>
          &larr; Adapt
        </a>
      </nav>

      {/* Header */}
      <header style={{ marginBottom: '2.5rem' }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'baseline',
            gap: '0.625rem',
            flexWrap: 'wrap',
            marginBottom: '0.5rem',
          }}
        >
          <span style={{ fontSize: '2rem', lineHeight: 1 }} aria-hidden="true">
            {asset.emoji}
          </span>
          <h1 style={{ margin: 0 }}>{asset.name}</h1>
          <span className="type-badge">{label}</span>
        </div>
      </header>

      {/* Description */}
      {asset.description && (
        <section style={{ marginBottom: '2rem' }}>
          <p style={{ margin: 0, lineHeight: 1.75 }}>{asset.description}</p>
        </section>
      )}

      {/* Triggers */}
      {asset.triggers && asset.triggers.length > 0 && (
        <section style={{ marginBottom: '2.5rem' }}>
          <h2
            style={{
              fontSize: '0.8125rem',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              color: 'var(--text-secondary)',
              marginBottom: '0.625rem',
            }}
          >
            Triggers
          </h2>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.375rem' }}>
            {asset.triggers.map((trigger) => (
              <span key={trigger} className="trigger-chip">
                {trigger}
              </span>
            ))}
          </div>
        </section>
      )}

      {/* Personality section — Roles and Colors */}
      {asset.personality && (
        <section style={{ marginBottom: '2.5rem' }}>
          <h2
            style={{
              fontSize: '0.8125rem',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              color: 'var(--text-secondary)',
              marginBottom: '1rem',
            }}
          >
            Personality
          </h2>

          {/* Motto */}
          {asset.personality.motto && (
            <blockquote
              style={{
                margin: '0 0 1.25rem',
                padding: '0.75rem 1rem',
                background: 'var(--bg-surface)',
                borderLeft: '3px solid var(--accent)',
                borderRadius: '0 6px 6px 0',
                fontStyle: 'italic',
                lineHeight: 1.65,
                fontSize: '0.9375rem',
              }}
            >
              {asset.personality.motto}
            </blockquote>
          )}

          {/* Principles */}
          {asset.personality.principles && asset.personality.principles.length > 0 && (
            <div style={{ marginBottom: '1rem' }}>
              <div
                style={{
                  fontSize: '0.8125rem',
                  fontWeight: 600,
                  color: 'var(--text-secondary)',
                  marginBottom: '0.5rem',
                  textTransform: 'uppercase',
                  letterSpacing: '0.04em',
                }}
              >
                Principles
              </div>
              <ul
                style={{
                  margin: 0,
                  paddingLeft: '1.25rem',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '0.3rem',
                }}
              >
                {asset.personality.principles.map((principle, i) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: static list
                  <li key={i} style={{ lineHeight: 1.65, fontSize: '0.9375rem' }}>
                    {principle}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Communication style */}
          {asset.personality.communicationStyle && (
            <div>
              <div
                style={{
                  fontSize: '0.8125rem',
                  fontWeight: 600,
                  color: 'var(--text-secondary)',
                  marginBottom: '0.25rem',
                  textTransform: 'uppercase',
                  letterSpacing: '0.04em',
                }}
              >
                Communication Style
              </div>
              <p style={{ margin: 0, lineHeight: 1.65, fontSize: '0.9375rem' }}>
                {asset.personality.communicationStyle}
              </p>
            </div>
          )}
        </section>
      )}

      {/* Approach section — Roles and Colors */}
      {asset.approach && <ApproachSection approach={asset.approach} />}

      {/* Anti-patterns section — Roles */}
      {asset.antiPatterns && asset.antiPatterns.length > 0 && (
        <section style={{ marginBottom: '2.5rem' }}>
          <h2
            style={{
              fontSize: '0.8125rem',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              color: 'var(--text-secondary)',
              marginBottom: '0.75rem',
            }}
          >
            Anti-patterns
          </h2>
          <ul
            style={{
              listStyle: 'none',
              margin: 0,
              padding: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: '0.375rem',
            }}
          >
            {asset.antiPatterns.map((pattern, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: static list
              <li key={i} className="anti-pattern-item">
                {pattern}
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Instruction section — Perspectives */}
      {asset.instruction && (
        <section style={{ marginBottom: '2.5rem' }}>
          <h2
            style={{
              fontSize: '0.8125rem',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              color: 'var(--text-secondary)',
              marginBottom: '0.75rem',
            }}
          >
            Instruction
          </h2>
          <p style={{ margin: 0, lineHeight: 1.75, fontSize: '0.9375rem' }}>
            {asset.instruction}
          </p>
        </section>
      )}
    </main>
  );
}
