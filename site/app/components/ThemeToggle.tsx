// T-SITE-001.3: RTM FR-15.31
'use client';

import { useEffect, useState } from 'react';

export function ThemeToggle() {
  const [isLight, setIsLight] = useState(false);

  useEffect(() => {
    // Read initial state from DOM (set by inline script in <head>)
    // Avoids hydration mismatch by not reading localStorage directly on first render.
    setIsLight(document.documentElement.classList.contains('light'));
  }, []);

  function toggle() {
    const next = !isLight;
    setIsLight(next);
    if (next) {
      document.documentElement.classList.add('light');
      localStorage.setItem('theme', 'light');
    } else {
      document.documentElement.classList.remove('light');
      localStorage.setItem('theme', 'dark');
    }
  }

  return (
    <button
      onClick={toggle}
      aria-label={isLight ? 'Switch to dark mode' : 'Switch to light mode'}
      title={isLight ? 'Switch to dark mode' : 'Switch to light mode'}
      style={{
        background: 'none',
        border: 'none',
        cursor: 'pointer',
        color: 'var(--text-secondary)',
        fontSize: '1.25rem',
        padding: '0.25rem',
      }}
    >
      {isLight ? '◐' : '◑'}
    </button>
  );
}
