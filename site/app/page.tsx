import Link from 'next/link';
import { GlassCard } from './components/GlassCard';
import { AnimatedText } from './components/AnimatedText';
import { RippleButton } from './components/RippleButton';

export default function Home() {
  return (
    <div className="relative h-screen flex items-center justify-center bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900 px-6">
      {/* Gradient mesh background (REQ-3.2) */}
      <div className="gradient-mesh-bg fixed inset-0 -z-20 opacity-60 dark:opacity-40" />
      <div className="max-w-6xl mx-auto text-center">
        {/* Hero Text */}
        <div className="mb-8">
          <h1 className="text-4xl md:text-5xl font-bold mb-3 animate-fade-in-up">
            <AnimatedText>Your agent is ephemeral.</AnimatedText>
          </h1>
          <h2 className="text-3xl md:text-4xl font-bold mb-4 text-slate-900 dark:text-white animate-fade-in-up animate-delay-100">
            Knowledge shouldn't be.
          </h2>
        </div>

        <p className="text-lg md:text-xl text-slate-700 dark:text-slate-300 mb-6 leading-relaxed max-w-3xl mx-auto animate-fade-in-up animate-delay-200">
          <strong>
            <AnimatedText>maenifold</AnimatedText>
          </strong>{' '}
          breaks the conversation boundary. Every tool contributes to the
          collective knowledge graph, linking each concept with every thought
          ever recorded about it—across sessions, across agents, across time.
        </p>

        {/* Feature Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8 max-w-5xl mx-auto animate-fade-in-up animate-delay-300">
          <GlassCard className="p-6">
            <div className="text-4xl mb-3">🔄</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              28 Workflows
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              Curated workflows for research and development - systematic
              methodologies for every cognitive task
            </p>
          </GlassCard>
          <GlassCard className="p-6">
            <div className="text-4xl mb-3">🎭</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              7 Roles
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              Professional perspectives from Product Manager to Red Team - each
              with personality, principles, and context-aware transitions
            </p>
          </GlassCard>
          <GlassCard className="p-6">
            <div className="text-4xl mb-3">🎩</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              7 Hats
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              De Bono's Six Thinking Hats plus Gray - cognitive modes for facts,
              emotions, caution, optimism, creativity, and skeptical inquiry
            </p>
          </GlassCard>
          <GlassCard className="p-6">
            <div className="text-4xl mb-3">🔌</div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-2">
              Dual Interface
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-400">
              Works as MCP server or CLI tool. Windows, Linux, MacOS supported.
            </p>
          </GlassCard>
        </div>

        <p className="text-base md:text-lg text-slate-600 dark:text-slate-400 mb-8 max-w-3xl mx-auto italic animate-fade-in-up animate-delay-400">
          Work your way: Let agents explore freely with fully extensivle roles
          and thinking modes, guide them through structured workflows, or
          orchestrate multiple agents sharing the same knowledge sessions.
          Everything flows into the graph—workflows spawn thinking sessions,
          thinking sessions link to concepts, and all of it persists.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-3 justify-center animate-fade-in-up animate-delay-400">
          <RippleButton
            href="/start"
            variant="primary"
            className="button-hover px-6 py-3 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 text-white font-bold rounded-lg"
          >
            Get Started
          </RippleButton>
          <RippleButton
            href="/tools"
            variant="secondary"
            className="button-hover px-6 py-3 border-2 border-blue-600 dark:border-blue-400 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-slate-800 font-bold rounded-lg"
          >
            Browse Tools
          </RippleButton>
          <RippleButton
            href="/docs/architecture"
            variant="tertiary"
            className="button-hover px-6 py-3 border-2 border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800 font-bold rounded-lg"
          >
            Documentation
          </RippleButton>
        </div>
      </div>
    </div>
  );
}
