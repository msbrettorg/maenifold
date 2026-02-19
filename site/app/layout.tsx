// T-SITE-001.3: RTM FR-15.31, FR-15.33
// T-SITE-001.20: Newsreader serif for heading typeface
import type { Metadata } from 'next';
import { Newsreader } from 'next/font/google';
import './globals.css';
import { Header } from './components/Header';
import { Footer } from './components/Footer';

const newsreader = Newsreader({
  subsets: ['latin'],
  display: 'swap',
  variable: '--font-heading',
  style: ['normal', 'italic'],
});

export const metadata: Metadata = {
  title: 'maenifold',
  description: 'Context engineering infrastructure for AI agents.',
};

// Inline script to prevent FOUC — runs before body renders.
// Theme cascade: localStorage → prefers-color-scheme → dark (default).
const themeScript = `
(function() {
  var stored = localStorage.getItem('theme');
  if (stored === 'light') {
    document.documentElement.classList.add('light');
  } else if (stored === 'dark') {
    // dark is default (no class needed)
  } else if (window.matchMedia('(prefers-color-scheme: light)').matches) {
    document.documentElement.classList.add('light');
  }
  // If none match, dark is the default (no class = dark mode)
})();
`;

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning className={newsreader.variable}>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body>
        {/* Skip link for keyboard nav (WCAG 2.4.1) */}
        <a
          href="#main-content"
          className="sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:rounded"
          style={{ backgroundColor: 'var(--accent)', color: 'var(--bg)' }}
        >
          Skip to content
        </a>
        <Header />
        <div id="main-content">
          {children}
        </div>
        <Footer />
      </body>
    </html>
  );
}
