// T-SITE-001.5 (PRD §7.9 Footer, FR-15.23)
import Link from 'next/link';
import { execSync } from 'node:child_process';

function getVersion(): string {
  try {
    return (
      execSync('git describe --tags --always', { stdio: ['ignore', 'pipe', 'ignore'] })
        .toString()
        .trim() || 'dev'
    );
  } catch {
    return 'dev';
  }
}

const VERSION = getVersion();

export default function Footer() {
  const linkStyle = { color: 'var(--color-accent)', textDecoration: 'none' } as const;
  return (
    <footer style={{ borderTop: '1px solid var(--color-border)', background: 'var(--color-bg-surface)', color: 'var(--color-text-secondary)' }}>
      <div style={{ maxWidth: 1120, margin: '0 auto', padding: '20px 16px', display: 'flex', gap: 16, alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', gap: 12, alignItems: 'center', flexWrap: 'wrap' }}>
          <Link href="/" style={{ display: 'inline-flex', gap: 8, alignItems: 'center', textDecoration: 'none', color: 'inherit' }}>
            <span aria-hidden style={{ width: 10, height: 10, borderRadius: 2, background: 'var(--color-accent)' }} />
          </Link>
          <span style={{ fontSize: 12 }}>v{VERSION}</span>
          <span style={{ fontSize: 12 }}>Domain expertise that compounds.</span>
        </div>
        <div style={{ display: 'flex', gap: 14, alignItems: 'center', fontSize: 12 }}>
          <a href="https://github.com/msbrettorg/maenifold/blob/main/LICENSE" target="_blank" rel="noopener noreferrer" style={linkStyle}>MIT</a>
          <a href="https://github.com/msbrettorg/maenifold" target="_blank" rel="noopener noreferrer" style={linkStyle}>GitHub</a>
        </div>
      </div>
    </footer>
  );
}
