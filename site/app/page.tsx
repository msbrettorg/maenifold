import Link from 'next/link';
import { GlassCard } from './components/GlassCard';
import { AnimatedText } from './components/AnimatedText';
import { RippleButton } from './components/RippleButton';
import { AnimatedGraph } from './components/AnimatedGraph';
import { CopyButton } from './components/CopyButton';

export default function Home() {
  return (
    <div className="relative bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
      {/* Gradient mesh background */}
      <div className="gradient-mesh-bg fixed inset-0 -z-20 opacity-60 dark:opacity-40" />

      {/* Animated network background */}
      <div className="knowledge-graph-bg fixed inset-0 -z-10 opacity-20 dark:opacity-10" />

      {/* Hero Section - Ephemeral Problem */}
      <section className="relative flex items-center justify-center px-6 py-32 overflow-hidden">
        {/* Animated graph canvas */}
        <div className="absolute inset-0 opacity-30 dark:opacity-20">
          <AnimatedGraph />
        </div>

        <div className="max-w-6xl mx-auto text-center relative z-10">
          {/* Dissolving particles effect */}
          <div className="relative mb-12">
            <div className="dissolve-particles absolute inset-0 pointer-events-none"></div>
            <h1 className="text-7xl md:text-9xl font-bold mb-6 animate-fade-in-up bg-gradient-to-r from-blue-600 via-purple-600 to-cyan-500 bg-clip-text text-transparent">
              maenifold
            </h1>
            <h2 className="text-3xl md:text-5xl font-semibold text-slate-700 dark:text-slate-300 animate-fade-in-up animate-delay-100">
              Never lose context.
            </h2>
          </div>

          {/* Install Options */}
          <div className="max-w-3xl mx-auto space-y-6 mb-12 animate-fade-in-up animate-delay-300">
            {/* NPM Install */}
            <div className="bg-slate-900 dark:bg-slate-950 rounded-xl p-6 border border-slate-700">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs font-mono text-slate-400">TERMINAL</span>
                <CopyButton
                  text="npm install -g maenifold"
                  className="text-xs text-blue-400 hover:text-blue-300 transition-colors"
                />
              </div>
              <code className="text-lg font-mono text-green-400 block">
                <span className="text-slate-500">$</span> npm install -g maenifold
              </code>
            </div>

            {/* VS Code Buttons */}
            <div className="flex flex-col sm:flex-row gap-3 justify-center">
              <a
                href="vscode:extension/mcp?install=maenifold"
                className="flex items-center justify-center gap-2 px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl"
              >
                <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"/>
                </svg>
                Install in VS Code
              </a>
              <a
                href="vscode-insiders:extension/mcp?install=maenifold"
                className="flex items-center justify-center gap-2 px-6 py-3 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl"
              >
                <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"/>
                </svg>
                VS Code Insiders
              </a>
            </div>

            {/* Or divider */}
            <div className="flex items-center gap-4">
              <div className="flex-1 h-px bg-slate-300 dark:bg-slate-700"></div>
              <span className="text-sm text-slate-500 dark:text-slate-400">or explore first</span>
              <div className="flex-1 h-px bg-slate-300 dark:bg-slate-700"></div>
            </div>

            {/* Secondary CTAs */}
            <div className="flex flex-col sm:flex-row gap-3 justify-center">
              <Link
                href="/start"
                className="px-6 py-3 border-2 border-blue-600 dark:border-blue-400 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-slate-800 rounded-lg font-semibold transition-all"
              >
                Quick Start Guide
              </Link>
              <Link
                href="/docs/architecture"
                className="px-6 py-3 border-2 border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800 rounded-lg font-semibold transition-all"
              >
                Documentation
              </Link>
            </div>
          </div>

          {/* Real Knowledge Graph Visualization */}
          <div className="max-w-4xl mx-auto mt-32 animate-fade-in-up animate-delay-400">
            <div className="text-center mb-8">
              <h3 className="text-4xl md:text-5xl font-bold text-slate-900 dark:text-white mb-4">
                maenifold's graph <span className="italic text-blue-600 dark:text-blue-400">is</span> the context window
              </h3>
              <p className="text-lg text-slate-600 dark:text-slate-400">
                Every <code className="px-2 py-1 bg-slate-100 dark:bg-slate-800 rounded text-blue-600 dark:text-blue-400">[[concept]]</code> becomes searchable. Every connection navigable.
              </p>
            </div>
            <div className="rounded-2xl overflow-hidden border-2 border-blue-200 dark:border-blue-900 shadow-2xl">
              <img
                src="/graph.jpeg"
                alt="Real knowledge graph showing interconnected concepts from maenifold development"
                className="w-full h-auto"
              />
            </div>
          </div>

          {/* Test-Time Reasoning Infrastructure */}
          <div className="max-w-6xl mx-auto mt-32 animate-fade-in-up animate-delay-500">
            <div className="text-center mb-16">
              <h3 className="text-4xl md:text-5xl font-bold mb-6 text-slate-900 dark:text-white">
                Test-time reasoning infrastructure
              </h3>
              <p className="text-xl text-slate-600 dark:text-slate-400 max-w-3xl mx-auto">
                Sequential thinking, workflow orchestration, and multi-agent coordination
              </p>
            </div>

            {/* Feature Cards */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
              <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
                <div className="text-5xl mb-4">🔄</div>
                <h4 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                  Sequential Thinking
                </h4>
                <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                  Multi-step reasoning with revision and branching. Test-time compute for systematic problem-solving.
                </p>
              </GlassCard>
              <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
                <div className="text-5xl mb-4">🎭</div>
                <h4 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                  30 Workflows
                </h4>
                <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                  From deductive reasoning to design thinking—systematic methodologies with quality gates.
                </p>
              </GlassCard>
              <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
                <div className="text-5xl mb-4">🎨</div>
                <h4 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                  Multi-Agent
                </h4>
                <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                  Coordinate agents in waves. PM preserves context while sub-agents execute.
                </p>
              </GlassCard>
              <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
                <div className="text-5xl mb-4">🔌</div>
                <h4 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                  Hybrid Search
                </h4>
                <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                  Semantic vectors + full-text with RRF fusion. Never miss exact matches or concepts.
                </p>
              </GlassCard>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="relative flex items-center justify-center px-6 py-32 border-t border-slate-200 dark:border-slate-800 overflow-hidden">
        {/* Animated graph background */}
        <div className="absolute inset-0 opacity-25 dark:opacity-20">
          <AnimatedGraph />
        </div>

        <div className="max-w-4xl mx-auto text-center relative z-10">
          <h2 className="text-3xl md:text-4xl font-bold mb-6 text-slate-900 dark:text-white">
            break the conversation boundary
          </h2>
          <p className="text-lg text-slate-600 dark:text-slate-400 mb-12 max-w-2xl mx-auto">
            Every session builds on the last. Every agent learns from every other.
            Knowledge persists, compounds, and evolves.
          </p>

          {/* CTA Buttons */}
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <RippleButton
              href="/start"
              variant="primary"
              className="button-hover px-8 py-4 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 text-white text-lg font-bold rounded-xl shadow-lg hover:shadow-xl transition-all"
            >
              Get Started
            </RippleButton>
            <RippleButton
              href="/tools"
              variant="secondary"
              className="button-hover px-8 py-4 border-2 border-blue-600 dark:border-blue-400 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-slate-800 text-lg font-bold rounded-xl transition-all"
            >
              Browse Tools
            </RippleButton>
            <RippleButton
              href="/docs/architecture"
              variant="tertiary"
              className="button-hover px-8 py-4 border-2 border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800 text-lg font-bold rounded-xl transition-all"
            >
              Documentation
            </RippleButton>
          </div>
        </div>
      </section>
    </div>
  );
}
