'use client';

import { useState, useEffect, Fragment } from 'react';
import Link from 'next/link';
import type { Route } from 'next';
import { Menu, Transition } from '@headlessui/react';

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

  // Close mobile menu on Escape key
  useEffect(() => {
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isMenuOpen) {
        setIsMenuOpen(false);
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [isMenuOpen]);

  // Focus trap for mobile menu
  useEffect(() => {
    if (isMenuOpen) {
      const menuElement = document.getElementById('mobile-menu');
      const focusableElements = menuElement?.querySelectorAll(
        'a, button, [tabindex]:not([tabindex="-1"])'
      );
      const firstElement = focusableElements?.[0] as HTMLElement;
      const lastElement = focusableElements?.[focusableElements.length - 1] as HTMLElement;

      const handleTab = (event: KeyboardEvent) => {
        if (event.key !== 'Tab') return;

        if (event.shiftKey) {
          if (document.activeElement === firstElement) {
            event.preventDefault();
            lastElement?.focus();
          }
        } else {
          if (document.activeElement === lastElement) {
            event.preventDefault();
            firstElement?.focus();
          }
        }
      };

      document.addEventListener('keydown', handleTab);
      return () => document.removeEventListener('keydown', handleTab);
    }
  }, [isMenuOpen]);

  return (
    <>
      {/* Skip link for keyboard navigation (WCAG 2.4.1) */}
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400"
      >
        Skip to main content
      </a>

      <header className="border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-slate-900 sticky top-0 z-50">
        <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-20">
            {/* Logo */}
            <div className="flex-shrink-0">
              <Link href="/" className="hover:opacity-80 transition-opacity">
                <img
                  src="/assets/branding/maenifold-logo.svg"
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

              {/* Use Cases Dropdown - Accessible with Headless UI */}
              <Menu as="div" className="relative">
                <Menu.Button className="text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white transition-colors flex items-center">
                  Use Cases
                  <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                  </svg>
                </Menu.Button>
                <Transition
                  as={Fragment}
                  enter="transition ease-out duration-100"
                  enterFrom="transform opacity-0 scale-95"
                  enterTo="transform opacity-100 scale-100"
                  leave="transition ease-in duration-75"
                  leaveFrom="transform opacity-100 scale-100"
                  leaveTo="transform opacity-0 scale-95"
                >
                  <Menu.Items className="absolute left-0 mt-2 w-64 bg-white dark:bg-slate-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg focus:outline-none">
                    <Menu.Item>
                      {({ active }) => (
                        <Link
                          href={'/use-cases/knowledge-foundation' as Route}
                          className={`block px-4 py-2 text-sm ${active ? 'bg-gray-100 dark:bg-slate-700' : ''} text-slate-700 dark:text-gray-300`}
                        >
                          📚 Research & Discovery
                        </Link>
                      )}
                    </Menu.Item>
                    <Menu.Item>
                      {({ active }) => (
                        <Link
                          href={'/use-cases/domain-extensibility' as Route}
                          className={`block px-4 py-2 text-sm ${active ? 'bg-gray-100 dark:bg-slate-700' : ''} text-slate-700 dark:text-gray-300`}
                        >
                          🔧 Domain Specialization
                        </Link>
                      )}
                    </Menu.Item>
                    <Menu.Item>
                      {({ active }) => (
                        <Link
                          href={'/use-cases/dev-work' as Route}
                          className={`block px-4 py-2 text-sm ${active ? 'bg-gray-100 dark:bg-slate-700' : ''} text-slate-700 dark:text-gray-300`}
                        >
                          💾 Institutional Memory
                        </Link>
                      )}
                    </Menu.Item>
                    <Menu.Item>
                      {({ active }) => (
                        <Link
                          href={'/use-cases/product-team' as Route}
                          className={`block px-4 py-2 text-sm ${active ? 'bg-gray-100 dark:bg-slate-700' : ''} text-slate-700 dark:text-gray-300`}
                        >
                          👥 Multi-Agent Collaboration
                        </Link>
                      )}
                    </Menu.Item>
                  </Menu.Items>
                </Transition>
              </Menu>

              {/* Docs Dropdown - Accessible with Headless UI */}
              <Menu as="div" className="relative">
                <Menu.Button className="text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white transition-colors flex items-center">
                  Docs
                  <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                  </svg>
                </Menu.Button>
                <Transition
                  as={Fragment}
                  enter="transition ease-out duration-100"
                  enterFrom="transform opacity-0 scale-95"
                  enterTo="transform opacity-100 scale-100"
                  leave="transition ease-in duration-75"
                  leaveFrom="transform opacity-100 scale-100"
                  leaveTo="transform opacity-0 scale-95"
                >
                  <Menu.Items className="absolute left-0 mt-2 w-48 bg-white dark:bg-slate-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg focus:outline-none">
                    <Menu.Item>
                      {({ active }) => (
                        <Link
                          href={'/docs/architecture' as Route}
                          className={`block px-4 py-2 text-sm ${active ? 'bg-gray-100 dark:bg-slate-700' : ''} text-slate-700 dark:text-gray-300`}
                        >
                          Architecture
                        </Link>
                      )}
                    </Menu.Item>
                    <Menu.Item>
                      {({ active }) => (
                        <Link
                          href={'/docs/technical-specs' as Route}
                          className={`block px-4 py-2 text-sm ${active ? 'bg-gray-100 dark:bg-slate-700' : ''} text-slate-700 dark:text-gray-300`}
                        >
                          Technical Specs
                        </Link>
                      )}
                    </Menu.Item>
                  </Menu.Items>
                </Transition>
              </Menu>

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
                href="https://github.com/msbrettorg/maenifold"
                target="_blank"
                rel="noopener noreferrer"
                className="p-2 rounded-lg bg-gray-100 dark:bg-slate-800 text-slate-900 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-slate-700 transition-colors"
                aria-label="GitHub"
              >
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                </svg>
              </a>

              {/* Mobile menu button */}
              <button
                onClick={() => setIsMenuOpen(!isMenuOpen)}
                className="md:hidden p-2 rounded-lg bg-gray-100 dark:bg-slate-800 text-slate-900 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-slate-700 transition-colors"
                aria-label="Toggle menu"
                aria-expanded={isMenuOpen}
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={isMenuOpen ? "M6 18L18 6M6 6l12 12" : "M4 6h16M4 12h16M4 18h16"} />
                </svg>
              </button>
            </div>
          </div>

          {/* Mobile Navigation with focus trap */}
          {isMenuOpen && (
            <div id="mobile-menu" className="md:hidden pb-4 border-t border-gray-200 dark:border-gray-800">
              <Link href={'/start' as Route} className="block px-4 py-2 text-slate-700 dark:text-gray-300 hover:text-slate-900 dark:hover:text-white">
                Start
              </Link>
              <div className="px-4 py-2">
                <div className="text-slate-700 dark:text-gray-300 font-medium mb-2">Use Cases</div>
                <Link href={'/use-cases/knowledge-foundation' as Route} className="block px-4 py-1 text-sm text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white">
                  📚 Research & Discovery
                </Link>
                <Link href={'/use-cases/domain-extensibility' as Route} className="block px-4 py-1 text-sm text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white">
                  🔧 Domain Specialization
                </Link>
                <Link href={'/use-cases/dev-work' as Route} className="block px-4 py-1 text-sm text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white">
                  💾 Institutional Memory
                </Link>
                <Link href={'/use-cases/product-team' as Route} className="block px-4 py-1 text-sm text-slate-600 dark:text-gray-400 hover:text-slate-900 dark:hover:text-white">
                  👥 Multi-Agent Collaboration
                </Link>
              </div>
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
    </>
  );
}
