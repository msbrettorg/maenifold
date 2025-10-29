import Link from 'next/link';
import { NetworkBackground } from './components/NetworkBackground';

export default function Home() {
  return (
    <div className="relative h-screen flex items-center justify-center bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900 px-6">
      {/* Gradient mesh background (REQ-3.2) */}
      <div className="gradient-mesh-bg fixed inset-0 -z-20 opacity-60 dark:opacity-40" />
      <NetworkBackground className="fixed inset-0 -z-10" />
      <div className="max-w-6xl mx-auto text-center">
        {/* Hero Text */}
        <div className="mb-8">
          <h1 className="text-4xl md:text-5xl font-bold mb-3 bg-gradient-to-r from-blue-600 to-purple-600 dark:from-blue-400 dark:to-purple-400 bg-clip-text text-transparent">
            Your agent is ephemeral.
          </h1>
          <h2 className="text-3xl md:text-4xl font-bold mb-4 text-slate-900 dark:text-white">
            Knowledge shouldn't be.
          </h2>
        </div>

        <p className="text-lg md:text-xl text-slate-700 dark:text-slate-300 mb-6 leading-relaxed max-w-3xl mx-auto">
          <strong className="bg-gradient-to-r from-blue-600 to-purple-600 dark:from-blue-400 dark:to-purple-400 bg-clip-text text-transparent">
            maenifold
          </strong>{' '}
          breaks the conversation boundary. Every tool contributes to the
          collective knowledge graph, linking each concept with every thought
          ever recorded about it—across sessions, across agents, across time.
        </p>

        {/* Feature Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8 max-w-5xl mx-auto">
          <div className="bg-white dark:bg-slate-800 p-6 rounded-lg shadow border border-slate-200 dark:border-slate-700">
            <div className="text-4xl mb-3">🔄</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              28 Workflows
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              Curated workflows for research and development - systematic
              methodologies for every cognitive task
            </p>
          </div>
          <div className="bg-white dark:bg-slate-800 p-6 rounded-lg shadow border border-slate-200 dark:border-slate-700">
            <div className="text-4xl mb-3">🎭</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              7 Roles
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              Professional perspectives from Product Manager to Red Team - each
              with personality, principles, and context-aware transitions
            </p>
          </div>
          <div className="bg-white dark:bg-slate-800 p-6 rounded-lg shadow border border-slate-200 dark:border-slate-700">
            <div className="text-4xl mb-3">🎩</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              7 Hats
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              De Bono's Six Thinking Hats plus Gray - cognitive modes for facts,
              emotions, caution, optimism, creativity, and skeptical inquiry
            </p>
          </div>
          <div className="bg-white dark:bg-slate-800 p-6 rounded-lg shadow border border-slate-200 dark:border-slate-700">
            <div className="text-4xl mb-3">🔌</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              Dual Interface
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              Works as MCP server or CLI tool. Windows, Linux, MacOS supported.
            </p>
          </div>
        </div>

        <p className="text-base md:text-lg text-slate-600 dark:text-slate-400 mb-8 max-w-3xl mx-auto italic">
          Fully extensible. Work your way: Let agents explore freely with roles
          and thinking modes, guide them through structured workflows, or
          orchestrate multiple agents sharing the same knowledge sessions.
          Everything flows into the graph—workflows spawn thinking sessions,
          thinking sessions link to concepts, and all of it persists.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-3 justify-center">
          <Link
            href="/start"
            className="px-6 py-3 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 text-white font-bold rounded-lg transition"
          >
            Get Started
          </Link>
          <Link
            href="/tools"
            className="px-6 py-3 border-2 border-blue-600 dark:border-blue-400 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-slate-800 font-bold rounded-lg transition"
          >
            Browse Tools
          </Link>
          <Link
            href="/docs/architecture"
            className="px-6 py-3 border-2 border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800 font-bold rounded-lg transition"
          >
            Documentation
          </Link>
        </div>
      </div>
    </div>
  );
}
