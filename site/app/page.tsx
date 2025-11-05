import Link from 'next/link';
import { GlassCard } from './components/GlassCard';
import { AnimatedText } from './components/AnimatedText';
import { RippleButton } from './components/RippleButton';
import { AnimatedGraph } from './components/AnimatedGraph';

export default function Home() {
  return (
    <div className="relative bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
      {/* Gradient mesh background */}
      <div className="gradient-mesh-bg fixed inset-0 -z-20 opacity-60 dark:opacity-40" />

      {/* Animated network background */}
      <div className="knowledge-graph-bg fixed inset-0 -z-10 opacity-20 dark:opacity-10" />

      {/* Hero Section - Ephemeral Problem */}
      <section className="relative min-h-screen flex items-center justify-center px-6 py-20 overflow-hidden">
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
            <h2 className="text-4xl md:text-6xl font-bold mb-8 text-white animate-fade-in-up animate-delay-100">
              break the context boundary
            </h2>
          </div>

          <p className="text-xl md:text-2xl text-slate-700 dark:text-slate-300 mb-12 leading-relaxed max-w-4xl mx-auto animate-fade-in-up animate-delay-200">
            <strong className="text-blue-600 dark:text-blue-400">
              <AnimatedText>maenifold</AnimatedText>
            </strong>{' '}
            transforms ephemeral AI sessions into persistent knowledge.
          </p>

          {/* Scroll indicator */}
          <div className="animate-bounce mt-16 opacity-50">
            <svg className="w-6 h-6 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
            </svg>
          </div>
        </div>
      </section>

      {/* The Graph is the Context Window - Concepts Materialize */}
      <section className="relative min-h-screen flex items-center justify-center px-6 py-20 border-t border-slate-200 dark:border-slate-800 bg-gradient-to-b from-transparent via-blue-50/30 to-transparent dark:via-slate-800/30 overflow-hidden">
        {/* Animated graph background */}
        <div className="absolute inset-0 opacity-20 dark:opacity-15">
          <AnimatedGraph />
        </div>

        <div className="max-w-6xl mx-auto relative z-10">
          <div className="text-center mb-16">
            <h2 className="text-4xl md:text-6xl font-bold mb-8 text-slate-900 dark:text-white">
              maenifold's <span className="italic text-blue-600 dark:text-blue-400">graph is</span> the context window
            </h2>
            <p className="text-xl text-slate-600 dark:text-slate-400 max-w-3xl mx-auto">
              Every <code className="px-2 py-1 bg-slate-100 dark:bg-slate-800 rounded text-blue-600 dark:text-blue-400">[[concept]]</code> becomes searchable.
              Every connection navigable.
            </p>
          </div>

          {/* WikiLink transformation visualization */}
          <div className="concept-transform-container relative max-w-5xl mx-auto mb-16 p-12 bg-slate-50 dark:bg-slate-800/50 rounded-2xl backdrop-blur-sm">
            <div className="concept-transform">
              <div className="concept-text text-2xl md:text-3xl text-slate-700 dark:text-slate-300 font-mono mb-8">
                <span className="opacity-60">I'm working with </span>
                <span className="wikilink-animate">[[maenifold]]</span>
                <span className="opacity-60">...</span>
              </div>

              {/* SVG Graph: What maenifold enables */}
              <svg className="concept-nodes w-full h-96" viewBox="0 0 1200 500">
                {/* Background glow */}
                <defs>
                  <radialGradient id="nodeGlow" cx="50%" cy="50%" r="50%">
                    <stop offset="0%" stopColor="rgb(59, 130, 246)" stopOpacity="0.4" />
                    <stop offset="100%" stopColor="rgb(59, 130, 246)" stopOpacity="0" />
                  </radialGradient>
                  <filter id="glow">
                    <feGaussianBlur stdDeviation="4" result="coloredBlur"/>
                    <feMerge>
                      <feMergeNode in="coloredBlur"/>
                      <feMergeNode in="SourceGraphic"/>
                    </feMerge>
                  </filter>
                </defs>

                {/* Connections from maenifold to dual purpose */}
                <line x1="150" y1="250" x2="400" y2="150" className="concept-connection" stroke="rgb(59, 130, 246)" strokeWidth="2.5" opacity="0.6" />
                <line x1="150" y1="250" x2="400" y2="350" className="concept-connection animation-delay-1" stroke="rgb(139, 92, 246)" strokeWidth="2.5" opacity="0.6" />

                {/* Connections from memory:// to capabilities */}
                <line x1="400" y1="150" x2="700" y2="100" className="concept-connection animation-delay-1" stroke="rgb(59, 130, 246)" strokeWidth="2" opacity="0.5" />
                <line x1="400" y1="150" x2="700" y2="250" className="concept-connection animation-delay-2" stroke="rgb(59, 130, 246)" strokeWidth="2" opacity="0.5" />

                {/* Connections from graph to capabilities */}
                <line x1="400" y1="350" x2="700" y2="250" className="concept-connection animation-delay-1" stroke="rgb(139, 92, 246)" strokeWidth="2" opacity="0.5" />
                <line x1="400" y1="350" x2="700" y2="400" className="concept-connection animation-delay-2" stroke="rgb(139, 92, 246)" strokeWidth="2" opacity="0.5" />

                {/* Connections from capabilities to outcomes */}
                <line x1="700" y1="100" x2="1000" y2="100" className="concept-connection animation-delay-2" stroke="rgb(168, 85, 247)" strokeWidth="2" opacity="0.5" />
                <line x1="700" y1="100" x2="1000" y2="250" className="concept-connection animation-delay-1" stroke="rgb(168, 85, 247)" strokeWidth="2" opacity="0.5" />
                <line x1="700" y1="250" x2="1000" y2="100" className="concept-connection animation-delay-2" stroke="rgb(168, 85, 247)" strokeWidth="1.5" opacity="0.4" />
                <line x1="700" y1="250" x2="1000" y2="250" className="concept-connection animation-delay-1" stroke="rgb(168, 85, 247)" strokeWidth="2" opacity="0.5" />
                <line x1="700" y1="250" x2="1000" y2="400" className="concept-connection animation-delay-2" stroke="rgb(168, 85, 247)" strokeWidth="1.5" opacity="0.4" />
                <line x1="700" y1="400" x2="1000" y2="250" className="concept-connection animation-delay-1" stroke="rgb(168, 85, 247)" strokeWidth="1.5" opacity="0.4" />
                <line x1="700" y1="400" x2="1000" y2="400" className="concept-connection animation-delay-2" stroke="rgb(168, 85, 247)" strokeWidth="2" opacity="0.5" />

                {/* TIER 1: Entry - maenifold */}
                <circle cx="150" cy="250" r="30" className="concept-node node-1" fill="url(#nodeGlow)" opacity="0.4" filter="url(#glow)" />
                <circle cx="150" cy="250" r="24" className="concept-node-pulse node-1" fill="none" stroke="rgb(59, 130, 246)" strokeWidth="2.5" />
                <text x="150" y="295" textAnchor="middle" className="text-sm fill-current font-bold">maenifold</text>

                {/* TIER 2: Dual Purpose - Trust (humans) + Context (agents) */}
                <circle cx="400" cy="150" r="28" className="concept-node node-2" fill="url(#nodeGlow)" opacity="0.4" filter="url(#glow)" />
                <circle cx="400" cy="150" r="22" className="concept-node-pulse node-2" fill="none" stroke="rgb(59, 130, 246)" strokeWidth="2.5" />
                <text x="400" y="125" textAnchor="middle" className="text-xs fill-current font-semibold text-blue-600 dark:text-blue-400">memory://</text>
                <text x="400" y="195" textAnchor="middle" className="text-xs fill-current font-normal text-slate-600 dark:text-slate-400">human observability</text>

                <circle cx="400" cy="350" r="28" className="concept-node node-3" fill="url(#nodeGlow)" opacity="0.4" filter="url(#glow)" />
                <circle cx="400" cy="350" r="22" className="concept-node-pulse node-3" fill="none" stroke="rgb(139, 92, 246)" strokeWidth="2.5" />
                <text x="400" y="325" textAnchor="middle" className="text-xs fill-current font-semibold text-purple-600 dark:text-purple-400">graph</text>
                <text x="400" y="395" textAnchor="middle" className="text-xs fill-current font-normal text-slate-600 dark:text-slate-400">agent context</text>

                {/* TIER 3: Capabilities Enabled */}
                <circle cx="700" cy="100" r="25" className="concept-node node-4" fill="url(#nodeGlow)" opacity="0.35" filter="url(#glow)" />
                <circle cx="700" cy="100" r="20" className="concept-node-pulse node-4" fill="none" stroke="rgb(168, 85, 247)" strokeWidth="2" />
                <text x="700" y="75" textAnchor="middle" className="text-xs fill-current font-semibold text-purple-600 dark:text-purple-400">sequential thinking</text>

                <circle cx="700" cy="250" r="25" className="concept-node node-5" fill="url(#nodeGlow)" opacity="0.35" filter="url(#glow)" />
                <circle cx="700" cy="250" r="20" className="concept-node-pulse node-5" fill="none" stroke="rgb(168, 85, 247)" strokeWidth="2" />
                <text x="700" y="285" textAnchor="middle" className="text-xs fill-current font-semibold text-purple-600 dark:text-purple-400">workflows</text>

                <circle cx="700" cy="400" r="25" className="concept-node node-6" fill="url(#nodeGlow)" opacity="0.35" filter="url(#glow)" />
                <circle cx="700" cy="400" r="20" className="concept-node-pulse node-6" fill="none" stroke="rgb(168, 85, 247)" strokeWidth="2" />
                <text x="700" y="435" textAnchor="middle" className="text-xs fill-current font-semibold text-purple-600 dark:text-purple-400">multi-agent</text>

                {/* TIER 4: Emergent Outcomes */}
                <circle cx="1000" cy="100" r="22" className="concept-node node-7" fill="url(#nodeGlow)" opacity="0.3" filter="url(#glow)" />
                <circle cx="1000" cy="100" r="17" className="concept-node-pulse node-7" fill="none" stroke="rgb(34, 211, 238)" strokeWidth="1.5" />
                <text x="1000" y="75" textAnchor="middle" className="text-xs fill-current font-semibold text-cyan-600 dark:text-cyan-400">knowledge-compounds</text>

                <circle cx="1000" cy="250" r="22" className="concept-node node-8" fill="url(#nodeGlow)" opacity="0.3" filter="url(#glow)" />
                <circle cx="1000" cy="250" r="17" className="concept-node-pulse node-8" fill="none" stroke="rgb(34, 211, 238)" strokeWidth="1.5" />
                <text x="1000" y="285" textAnchor="middle" className="text-xs fill-current font-semibold text-cyan-600 dark:text-cyan-400">emergent-intelligence</text>

                <circle cx="1000" cy="400" r="22" className="concept-node node-9" fill="url(#nodeGlow)" opacity="0.3" filter="url(#glow)" />
                <circle cx="1000" cy="400" r="17" className="concept-node-pulse node-9" fill="none" stroke="rgb(34, 211, 238)" strokeWidth="1.5" />
                <text x="1000" y="435" textAnchor="middle" className="text-xs fill-current font-semibold text-cyan-600 dark:text-cyan-400">infinite-context</text>
              </svg>
            </div>
          </div>
        </div>
      </section>

      {/* Work Your Way Section */}
      <section className="relative min-h-screen flex items-center justify-center px-6 py-20 border-t border-slate-200 dark:border-slate-800 overflow-hidden">
        {/* Animated graph background */}
        <div className="absolute inset-0 opacity-20 dark:opacity-15">
          <AnimatedGraph />
        </div>

        <div className="max-w-6xl mx-auto relative z-10">
          <div className="text-center mb-16">
            <h2 className="text-4xl md:text-5xl font-bold mb-6 text-slate-900 dark:text-white">
              Work Your Way
            </h2>
            <p className="text-xl text-slate-600 dark:text-slate-400 max-w-3xl mx-auto">
              Master knowledge domains with extensible tools as the graph evolves
            </p>
          </div>

          {/* Feature Cards */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-16">
            <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
              <div className="text-5xl mb-4">🔄</div>
              <h3 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                32 Workflows
              </h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                From design thinking to game theory—systematic methodologies for every cognitive task
              </p>
            </GlassCard>
            <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
              <div className="text-5xl mb-4">🎭</div>
              <h3 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                16 Roles
              </h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                Professional perspectives with personality, principles, and context-aware transitions
              </p>
            </GlassCard>
            <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
              <div className="text-5xl mb-4">🎨</div>
              <h3 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                19 Perspectives
              </h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                De Bono's thinking hats plus linguistic perspectives—cognitive modes and cultural lenses
              </p>
            </GlassCard>
            <GlassCard className="p-8 hover:scale-105 transition-all duration-300">
              <div className="text-5xl mb-4">🔌</div>
              <h3 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
                Dual Interface
              </h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                MCP server or CLI tool. Windows, Linux, MacOS supported.
              </p>
            </GlassCard>
          </div>

          {/* Flow visualization */}
          <div className="max-w-4xl mx-auto p-8 bg-gradient-to-r from-blue-50 to-purple-50 dark:from-slate-800 dark:to-slate-700 rounded-2xl border border-blue-200 dark:border-slate-600">
            <div className="flex flex-wrap items-center justify-center gap-4 text-lg md:text-xl font-semibold text-slate-700 dark:text-slate-200">
              <span className="px-4 py-2 bg-white dark:bg-slate-600 rounded-lg shadow">Learn</span>
              <span className="text-blue-500">→</span>
              <span className="px-4 py-2 bg-white dark:bg-slate-600 rounded-lg shadow">Customize</span>
              <span className="text-purple-500">→</span>
              <span className="px-4 py-2 bg-white dark:bg-slate-600 rounded-lg shadow">Work</span>
              <span className="text-green-500">→</span>
              <span className="px-4 py-2 bg-white dark:bg-slate-600 rounded-lg shadow animate-pulse">Profit</span>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="relative min-h-[60vh] flex items-center justify-center px-6 py-20 border-t border-slate-200 dark:border-slate-800 overflow-hidden">
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
