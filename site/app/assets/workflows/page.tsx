import { getAllWorkflows } from '@/lib/assets';
import { GlassCard } from '@/app/components/GlassCard';

export default function WorkflowsPage() {
  const workflows = getAllWorkflows();

  // Categorize workflows based on the README groupings
  const categories = {
    'Thinking & Reasoning': workflows.filter(w =>
      ['abductive-reasoning', 'convergent-thinking', 'critical-thinking', 'data-thinking',
       'deductive-reasoning', 'design-thinking', 'divergent-thinking', 'higher-order-thinking',
       'inductive-reasoning', 'lateral-thinking', 'parallel-thinking', 'strategic-thinking'].includes(w.id)
    ),
    'Multi-Agent Orchestrated': workflows.filter(w =>
      ['agentic-research', 'agentic-slc', 'game-theory', 'think-tank'].includes(w.id)
    ),
    'Development Methodologies': workflows.filter(w =>
      ['agile', 'agentic-dev', 'lean-startup', 'sdlc'].includes(w.id)
    ),
    'Creative Problem Solving': workflows.filter(w =>
      ['oblique-strategies', 'provocative-operation', 'scamper'].includes(w.id)
    ),
    'Structured Problem Solving': workflows.filter(w =>
      ['socratic-dialogue', 'polya-problem-solving'].includes(w.id)
    ),
    'Collaborative Processes': workflows.filter(w =>
      ['sixhat', 'world-cafe'].includes(w.id)
    ),
    'FinOps': workflows.filter(w =>
      ['ftk-query', 'ftk-analysis'].includes(w.id)
    ),
    'Business Strategy': workflows.filter(w =>
      ['crta'].includes(w.id)
    ),
    'Meta/System': workflows.filter(w =>
      ['role-creation-workflow', 'workflow-dispatch'].includes(w.id)
    ),
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900 px-6 py-16">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="text-center mb-12">
          <h1 className="text-4xl md:text-5xl font-bold mb-4 text-slate-900 dark:text-white">
            🔄 Workflows
          </h1>
          <p className="text-lg text-slate-700 dark:text-slate-300 max-w-3xl mx-auto">
            32 curated workflows for systematic thinking and problem-solving. From research methodologies
            to multi-agent orchestration, each workflow provides structured guidance for complex cognitive tasks.
          </p>
        </div>

        {/* Categories */}
        {Object.entries(categories).map(([category, items]) => {
          if (items.length === 0) return null;

          return (
            <div key={category} className="mb-12">
              <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
                {category} ({items.length})
              </h2>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {items.map((workflow) => (
                  <GlassCard key={workflow.id} className="p-6 hover:scale-105 transition-transform">
                    <div className="flex items-start gap-3 mb-3">
                      <div className="text-3xl">{workflow.emoji}</div>
                      <div className="flex-1">
                        <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-1">
                          {workflow.name}
                        </h3>
                        <p className="text-sm text-slate-600 dark:text-slate-400">
                          {workflow.description}
                        </p>
                      </div>
                    </div>
                    {workflow.enhancedThinkingEnabled && (
                      <div className="mt-3 pt-3 border-t border-slate-200 dark:border-slate-700">
                        <span className="text-xs font-semibold text-blue-600 dark:text-blue-400">
                          🧠 Enhanced Thinking Enabled
                        </span>
                      </div>
                    )}
                    {workflow.steps && workflow.steps.length > 0 && (
                      <div className="mt-3 pt-3 border-t border-slate-200 dark:border-slate-700">
                        <span className="text-xs text-slate-500 dark:text-slate-500">
                          {workflow.steps.length} steps
                        </span>
                      </div>
                    )}
                  </GlassCard>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
