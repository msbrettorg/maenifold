'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import type { Route } from 'next';

export default function Header() {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);

  // Initialize dark mode from localStorage
  useEffect(() => {
    const savedDarkMode = localStorage.getItem('darkMode') === 'true';
    setIsDarkMode(savedDarkMode);
    if (savedDarkMode) {
      document.documentElement.classList.add('dark');
    }
  }, []);

  // Toggle dark mode
  const toggleDarkMode = () => {
    const newDarkMode = !isDarkMode;
    setIsDarkMode(newDarkMode);
    localStorage.setItem('darkMode', newDarkMode.toString());

    if (newDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  };

  return (
    <header className="border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-slate-900 sticky top-0 z-50">
      <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-20">
          {/* Logo */}
          <div className="flex-shrink-0">
            <Link href="/" className="hover:opacity-80 transition-opacity">
              <img
                src="/maenifold/assets/branding/maenifold-logo.svg"
                alt="maenifold"
                className="h-16 w-auto"
                style={{ minWidth: '200px' }}
              />
            </Link>
          </div>

          {/* Desktop Navigation */}
          <div className="hidden md:flex items-center space-x-8">
            <Link href={'/start' as Route} className="text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white transition-colors">
              Start
            </Link>

            {/* Docs Dropdown */}
            <div className="relative group">
              <button className="text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white transition-colors flex items-center">
                Docs
                <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
                </svg>
              </button>
              <div className="absolute left-0 mt-0 w-48 bg-white dark:bg-slate-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200">
                <Link href={'/docs/architecture' as Route} className="block px-4 py-2 text-sm text-slate-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-slate-700">
                  Architecture
                </Link>
                <Link href={'/docs/technical-specs' as Route} className="block px-4 py-2 text-sm text-slate-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-slate-700">
                  Technical Specs
                </Link>
              </div>
            </div>

            <Link href={'/tools' as Route} className="text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white transition-colors">
              Tools
            </Link>
          </div>

          {/* Right side: Dark mode toggle and GitHub */}
          <div className="flex items-center space-x-4">
            <button
              onClick={toggleDarkMode}
              className="p-2 rounded-lg bg-gray-100 dark:bg-slate-800 text-slate-900 dark:text-yellow-400 hover:bg-gray-200 dark:hover:bg-slate-700 transition-colors"
              aria-label="Toggle dark mode"
            >
              {isDarkMode ? (
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
              href="https://github.com/maenifold/maenifold"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 rounded-lg bg-gray-100 dark:bg-slate-800 text-slate-900 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-slate-700 transition-colors"
              aria-label="GitHub"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v 3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
              </svg>
            </a>

            {/* Mobile menu button */}
            <button
              onClick={() => setIsMenuOpen(!isMenuOpen)}
              className="md:hidden p-2 rounded-lg bg-gray-100 dark:bg-slate-800 text-slate-900 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-slate-700 transition-colors"
              aria-label="Toggle menu"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={isMenuOpen ? "M6 18L18 6M6 6l12 12" : "M4 6h16M4 12h16M4 18h16"} />
              </svg>
            </button>
          </div>
        </div>

        {/* Mobile Navigation */}
        {isMenuOpen && (
          <div className="md:hidden pb-4 border-t border-gray-200 dark:border-gray-800">
            <Link href={'/start' as Route} className="block px-4 py-2 text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white">
              Start
            </Link>
            <div className="px-4 py-2">
              <div className="text-slate-700 dark:text-gray-300 font-medium mb-2">Docs</div>
              <Link href={'/docs/architecture' as Route} className="block px-4 py-1 text-sm text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white">
                Architecture
              </Link>
              <Link href={'/docs/technical-specs' as Route} className="block px-4 py-1 text-sm text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white">
                Technical Specs
              </Link>
            </div>
            <Link href={'/tools' as Route} className="block px-4 py-2 text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white">
              Tools
            </Link>
          </div>
        )}
      </nav>
    </header>
  );
}
