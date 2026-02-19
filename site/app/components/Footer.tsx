// T-SITE-001.5: RTM FR-15.23
import { VERSION } from '../../lib/version';

export function Footer() {
  return (
    <footer
      style={{
        borderTop: '1px solid var(--border)',
        color: 'var(--text-secondary)',
        padding: '3rem 1rem',
      }}
    >
      <div
        className="prose-width"
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '0.75rem',
          textAlign: 'center',
        }}
      >
        {/* Logo + version */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <img src="/maenifold-logo.svg" alt="maenifold" style={{ height: '20px' }} />
          <span style={{ fontSize: '0.875rem' }}>{VERSION}</span>
        </div>

        {/* Brand statement */}
        <p style={{ margin: 0, fontSize: '0.875rem' }}>
          Domain expertise that compounds.
        </p>

        {/* Links */}
        <div style={{ display: 'flex', gap: '1rem', fontSize: '0.875rem' }}>
          <a
            href="https://github.com/msbrettorg/maenifold/blob/main/LICENSE"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: 'var(--accent)' }}
          >
            MIT License
          </a>
          <span aria-hidden="true">|</span>
          <a
            href="https://github.com/msbrettorg/maenifold"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: 'var(--accent)' }}
          >
            GitHub
          </a>
        </div>
      </div>
    </footer>
  );
}
