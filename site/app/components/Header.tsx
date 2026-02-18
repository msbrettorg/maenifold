// T-SITE-001.4
'use client';

import Link from 'next/link';
import type { Route } from 'next';
import type { CSSProperties } from 'react';
import { useEffect, useState } from 'react';

type Theme = 'light' | 'dark';

function applyTheme(theme: Theme) {
  document.documentElement.classList.remove('dark', 'light');
  document.documentElement.classList.add(theme);
}

export default function Header() {
  const [theme, setTheme] = useState<Theme>('dark');

  useEffect(() => {
    const saved = localStorage.getItem('theme');
    const initial: Theme = saved === 'light' || saved === 'dark'
      ? saved
      : (document.documentElement.classList.contains('light') ? 'light' : 'dark');
    setTheme(initial);
    applyTheme(initial);
  }, []);

  const toggleTheme = () => {
    const next: Theme = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    localStorage.setItem('theme', next);
    applyTheme(next);
  };

  const chromeStyle: CSSProperties = {
    backgroundColor: 'var(--color-bg-surface)',
    borderBottom: '1px solid var(--color-border)',
    color: 'var(--color-text)',
  };

  const iconButtonStyle: CSSProperties = {
    border: '1px solid var(--color-border)',
    color: 'var(--color-text)',
  };

  return (
    <header className="sticky top-0 z-50" style={chromeStyle}>
      <nav className="mx-auto max-w-6xl px-4 py-3" style={{ overflowX: 'auto', WebkitOverflowScrolling: 'touch' }}>
        <div className="flex items-center gap-4 whitespace-nowrap">
          <Link href="/" className="shrink-0 opacity-90 hover:opacity-100 transition-opacity">
            <img src="/assets/branding/maenifold-logo.svg" alt="maenifold" className="h-8 w-auto" />
          </Link>

          <Link href={'/docs' as Route} className="text-sm hover:opacity-80" style={{ color: 'var(--color-text)' }}>Docs</Link>
          <Link href={'/plugins' as Route} className="text-sm hover:opacity-80" style={{ color: 'var(--color-text)' }}>Plugins</Link>
          <Link href={'/tools' as Route} className="text-sm hover:opacity-80" style={{ color: 'var(--color-text)' }}>Tools</Link>
          <Link href={'/workflows' as Route} className="text-sm hover:opacity-80" style={{ color: 'var(--color-text)' }}>Workflows</Link>

          <div className="ml-auto flex items-center gap-2">
            <button
              type="button"
              onClick={toggleTheme}
              className="p-2 rounded-md hover:opacity-80 transition-opacity"
              style={iconButtonStyle}
              aria-label="Toggle theme"
            >
              {theme === 'dark' ? (
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z" />
                </svg>
              ) : (
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 2a1 1 0 011 1v1a1 1 0 11-2 0V3a1 1 0 011-1zm4 8a4 4 0 11-8 0 4 4 0 018 0zm-.464 4.536l.707.707a1 1 0 001.414-1.414l-.707-.707a1 1 0 00-1.414 1.414zm2.121-10.607a1 1 0 010 1.414l-.707.707a1 1 0 11-1.414-1.414l.707-.707a1 1 0 011.414 0zM7 11a1 1 0 100-2H6a1 1 0 100 2h1zm-4.536-.464a1 1 0 011.414 0l.707.707a1 1 0 11-1.414 1.414l-.707-.707a1 1 0 010-1.414zM3 8a1 1 0 110 2H2a1 1 0 110-2h1z" clipRule="evenodd" />
                </svg>
              )}
            </button>

            <a
              href="https://github.com/msbrettorg/maenifold"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 rounded-md hover:opacity-80 transition-opacity"
              style={iconButtonStyle}
              aria-label="GitHub"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
              </svg>
            </a>
          </div>
        </div>
      </nav>
    </header>
  );
}
