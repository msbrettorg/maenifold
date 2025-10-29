import Link from 'next/link';
import { GlassCard } from '../components/GlassCard';

export default function UseCasesPage() {
  return (
    <div className="max-w-6xl mx-auto px-4 py-12">
      <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">Use Cases</h1>
      <p className="text-lg text-slate-600 dark:text-slate-300 mb-12">
        Four proven patterns for integrating maenifold into your workflow—from knowledge building
        to multi-agent orchestration.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* UC1: Knowledge Foundation */}
        <Link href="/use-cases/knowledge-foundation">
          <GlassCard className="p-6 h-full hover:border-blue-500 dark:hover:border-blue-400 transition-colors cursor-pointer">
            <div className="flex items-start gap-4">
              <div className="text-4xl">📚</div>
              <div>
                <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">
                  Research & Discovery
                </h2>
                <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">
                  Seed the graph with domain knowledge
                </p>
                <p className="text-sm text-slate-700 dark:text-slate-300">
                  Use <strong>agentic-research</strong> or <strong>think-tank</strong> workflows
                  to research any domain and build a comprehensive knowledge graph. Perfect for
                  starting new projects or exploring unfamiliar territories.
                </p>
                <div className="mt-4 text-sm text-blue-600 dark:text-blue-400 font-semibold">
                  Single or Multi-Agent →
                </div>
              </div>
            </div>
          </GlassCard>
        </Link>

        {/* UC2: Domain Extensibility */}
        <Link href="/use-cases/domain-extensibility">
          <GlassCard className="p-6 h-full hover:border-blue-500 dark:hover:border-blue-400 transition-colors cursor-pointer">
            <div className="flex items-start gap-4">
              <div className="text-4xl">🔧</div>
              <div>
                <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">
                  Domain Specialization
                </h2>
                <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">
                  AI creates its own expertise
                </p>
                <p className="text-sm text-slate-700 dark:text-slate-300">
                  AI studies your UC1 knowledge and external docs to create custom roles, workflows,
                  and domain-specific tooling. Demonstrated with <strong>$323K ROI</strong> from
                  FinOps analysis.
                </p>
                <div className="mt-4 text-sm text-blue-600 dark:text-blue-400 font-semibold">
                  AI-Generated Expertise →
                </div>
              </div>
            </div>
          </GlassCard>
        </Link>

        {/* UC3: Development Memory Bank */}
        <Link href="/use-cases/dev-work">
          <GlassCard className="p-6 h-full hover:border-blue-500 dark:hover:border-blue-400 transition-colors cursor-pointer">
            <div className="flex items-start gap-4">
              <div className="text-4xl">💾</div>
              <div>
                <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">
                  Institutional Memory
                </h2>
                <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">
                  Persistent context across sessions
                </p>
                <p className="text-sm text-slate-700 dark:text-slate-300">
                  Never lose context between coding sessions. Codex + maenifold creates persistent
                  project memory with decisions, architecture, and progress linked through the
                  knowledge graph.
                </p>
                <div className="mt-4 text-sm text-blue-600 dark:text-blue-400 font-semibold">
                  Single Agent Workflows →
                </div>
              </div>
            </div>
          </GlassCard>
        </Link>

        {/* UC4: Multi-Agent Orchestration */}
        <Link href="/use-cases/product-team">
          <GlassCard className="p-6 h-full hover:border-blue-500 dark:hover:border-blue-400 transition-colors cursor-pointer">
            <div className="flex items-start gap-4">
              <div className="text-4xl">👥</div>
              <div>
                <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">
                  Multi-Agent Collaboration
                </h2>
                <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">
                  Simulate an entire product team
                </p>
                <p className="text-sm text-slate-700 dark:text-slate-300">
                  Deploy <strong>25 agents</strong> in parallel waves: PM, engineers, QA, red team—all
                  sharing a single knowledge graph. Real demo: 68 minutes, 4 bugs fixed, 2,031 test lines added.
                </p>
                <div className="mt-4 text-sm text-blue-600 dark:text-blue-400 font-semibold">
                  Multi-Agent Orchestration →
                </div>
              </div>
            </div>
          </GlassCard>
        </Link>
      </div>

      {/* Progressive Sophistication Note */}
      <div className="mt-12 p-6 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
        <h3 className="text-xl font-bold text-slate-900 dark:text-white mb-3">
          Progressive Sophistication
        </h3>
        <p className="text-slate-700 dark:text-slate-300 mb-3">
          These use cases demonstrate maenifold's flexibility—<strong>you don't need multi-agent
          orchestration to benefit</strong>. Start with single-agent research (UC1), extend to your
          domain (UC2), use it daily for development (UC3), and scale to multi-agent when needed (UC4).
        </p>
        <p className="text-sm text-slate-600 dark:text-slate-400">
          The knowledge graph connects everything: workflows spawn thinking sessions, thinking sessions
          link concepts, and all of it persists across agents and time.
        </p>
      </div>
    </div>
  );
}
