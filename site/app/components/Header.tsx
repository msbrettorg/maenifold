// T-SITE-001.4: RTM FR-15.32, FR-15.35
'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { ThemeToggle } from './ThemeToggle';

const NAV_LINKS = [
  { href: '/docs', label: 'Docs' },
  { href: '/plugins', label: 'Plugins' },
  { href: '/tools', label: 'Tools' },
  { href: '/workflows', label: 'Workflows' },
] as const;

export function Header() {
  const pathname = usePathname();

  return (
    <header
      style={{
        position: 'sticky',
        top: 0,
        zIndex: 40,
        backgroundColor: 'var(--bg-surface)',
        borderBottom: '1px solid var(--border)',
        overflowX: 'auto',
      }}
    >
      <div
        style={{
          maxWidth: '900px',
          margin: '0 auto',
          padding: '0 1rem',
          display: 'flex',
          flexDirection: 'row',
          alignItems: 'center',
          gap: '1.5rem',
          height: '52px',
          whiteSpace: 'nowrap',
        }}
      >
        {/* Logo */}
        <Link
          href="/"
          style={{ display: 'flex', alignItems: 'center', flexShrink: 0 }}
          aria-label="maenifold home"
        >
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src="/maenifold-logo.svg"
            alt="maenifold"
            width={58}
            height={24}
            style={{ height: '24px', display: 'block' }}
          />
        </Link>

        {/* Page links */}
        <nav
          aria-label="Main navigation"
          style={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '1.25rem',
            flexShrink: 0,
          }}
        >
          {NAV_LINKS.map(({ href, label }) => {
            const isActive = pathname === href || pathname.startsWith(href + '/');
            return (
              <Link
                key={href}
                href={href}
                style={{
                  color: isActive ? 'var(--accent)' : 'var(--text-secondary)',
                  textDecoration: 'none',
                  fontSize: '0.9375rem',
                  fontWeight: isActive ? 500 : 400,
                  transition: 'color 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  if (!isActive) {
                    (e.currentTarget as HTMLAnchorElement).style.color = 'var(--text)';
                  }
                }}
                onMouseLeave={(e) => {
                  if (!isActive) {
                    (e.currentTarget as HTMLAnchorElement).style.color = 'var(--text-secondary)';
                  }
                }}
              >
                {label}
              </Link>
            );
          })}
        </nav>

        {/* Spacer — pushes right-side controls to the end */}
        <div style={{ flex: 1, minWidth: '0.5rem' }} />

        {/* Right-side controls */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '1rem',
            flexShrink: 0,
          }}
        >
          <ThemeToggle />
          <a
            href="https://github.com/msbrettorg/maenifold"
            target="_blank"
            rel="noopener noreferrer"
            style={{
              color: 'var(--text-secondary)',
              textDecoration: 'none',
              fontSize: '0.9375rem',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLAnchorElement).style.color = 'var(--text)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLAnchorElement).style.color = 'var(--text-secondary)';
            }}
          >
            GitHub
          </a>
        </div>
      </div>
    </header>
  );
}
