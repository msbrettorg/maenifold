import Link from 'next/link';
import type { Route } from 'next';

export default function Footer() {
  const currentYear = new Date().getFullYear();

  return (
    <footer className="border-t border-gray-200 dark:border-gray-800 bg-white dark:bg-slate-900">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8 mb-8">
          {/* About */}
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-white uppercase tracking-wider mb-4">
              About
            </h3>
            <ul className="space-y-3">
              <li>
                <Link href="/" className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  Home
                </Link>
              </li>
              <li>
                <Link href={'/start' as Route} className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  Quick Start
                </Link>
              </li>
              <li>
                <a href="https://github.com/msbrettorg/maenifold" target="_blank" rel="noopener noreferrer" className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  GitHub
                </a>
              </li>
            </ul>
          </div>

          {/* Documentation */}
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-white uppercase tracking-wider mb-4">
              Documentation
            </h3>
            <ul className="space-y-3">
              <li>
                <Link href={'/docs/architecture' as Route} className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  Architecture
                </Link>
              </li>
              <li>
                <Link href={'/docs/technical-specs' as Route} className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  Technical Specs
                </Link>
              </li>
            </ul>
          </div>

          {/* Resources */}
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-white uppercase tracking-wider mb-4">
              Resources
            </h3>
            <ul className="space-y-3">
              <li>
                <Link href={'/tools' as Route} className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  Tools
                </Link>
              </li>
            </ul>
          </div>

          {/* Legal */}
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-white uppercase tracking-wider mb-4">
              Legal
            </h3>
            <ul className="space-y-3">
              <li>
                <span className="text-slate-600 dark:text-gray-400">
                  MIT License
                </span>
              </li>
              <li>
                <a href="https://github.com/msbrettorg/maenifold/blob/main/LICENSE" target="_blank" rel="noopener noreferrer" className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors">
                  License Details
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom section */}
        <div className="border-t border-gray-200 dark:border-gray-800 pt-8">
          <div className="flex flex-col md:flex-row justify-between items-center">
            <div className="text-slate-600 dark:text-gray-400 text-sm mb-4 md:mb-0">
              <p>
                &copy; {currentYear} Maenifold. Built with the Ma Protocol.
              </p>
            </div>

            {/* Version badge */}
            <div className="flex items-center space-x-4">
              <span className="inline-block px-3 py-1 text-xs font-medium bg-slate-100 dark:bg-slate-800 text-slate-900 dark:text-gray-300 rounded-full">
                v1.0.0
              </span>
              <a
                href="https://github.com/msbrettorg/maenifold"
                target="_blank"
                rel="noopener noreferrer"
                className="text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white transition-colors"
                aria-label="GitHub"
              >
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v 3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                </svg>
              </a>
            </div>
          </div>
        </div>
      </div>
    </footer>
  );
}
