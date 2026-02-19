// T-SITE-001.21: Sticky sidebar TOC for docs pages
// Client component — extracts headings from rendered DOM, highlights on scroll
'use client';

import { useEffect, useState } from 'react';

interface TocEntry {
  id: string;
  text: string;
  level: number;
}

export function DocsSidebar() {
  const [headings, setHeadings] = useState<TocEntry[]>([]);
  const [activeId, setActiveId] = useState<string>('');

  useEffect(() => {
    const content = document.getElementById('docs-content');
    if (!content) return;

    // h2 only — the README has ~35 h2+h3 headings; showing all is noise
    const elements = content.querySelectorAll('h2[id]');
    const entries: TocEntry[] = [];
    elements.forEach(el => {
      entries.push({
        id: el.id,
        text: el.textContent || '',
        level: 2,
      });
    });
    setHeadings(entries);

    // Highlight the heading closest to the top of the viewport
    const observer = new IntersectionObserver(
      (observed) => {
        for (const entry of observed) {
          if (entry.isIntersecting) {
            setActiveId(entry.target.id);
          }
        }
      },
      { rootMargin: '-80px 0px -75% 0px' },
    );

    elements.forEach(el => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  if (headings.length === 0) return null;

  return (
    <nav aria-label="Table of contents" className="docs-sidebar">
      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {headings.map(h => (
          <li key={h.id}>
            <a
              href={`#${h.id}`}
              className={activeId === h.id ? 'docs-toc-active' : ''}
              style={{
                display: 'block',
                fontSize: '0.8125rem',
                lineHeight: 1.5,
                padding: '0.25rem 0 0.25rem 0.75rem',
                textDecoration: 'none',
                color: activeId === h.id ? 'var(--accent)' : 'var(--text-secondary)',
                borderLeft: `2px solid ${activeId === h.id ? 'var(--accent)' : 'transparent'}`,
                transition: 'color 0.15s ease, border-color 0.15s ease',
              }}
            >
              {h.text}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}
