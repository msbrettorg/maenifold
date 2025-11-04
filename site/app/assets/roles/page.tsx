import { getAllRoles } from '@/lib/assets';
import { GlassCard } from '@/app/components/GlassCard';

export default function RolesPage() {
  const roles = getAllRoles();

  // Categorize roles based on the README groupings (alphabetically sorted)
  const categories = {
    'AI': roles.filter(r =>
      ['prompt-engineer', 'prompt-engineer-gpt5', 'prompt-engineer-codex'].includes(r.id)
    ),
    'EDA (Electronic Design Automation)': roles.filter(r =>
      ['eda-architect', 'eda-platform-engineer'].includes(r.id)
    ),
    'FinOps': roles.filter(r =>
      ['cfo', 'finops-practitioner', 'ftk-agent'].includes(r.id)
    ),
    'Research': roles.filter(r =>
      ['researcher', 'writer'].includes(r.id)
    ),
    'Software': roles.filter(r =>
      ['architect', 'engineer', 'mcp-specialist', 'product-manager', 'red-team', 'blue-team'].includes(r.id)
    ),
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900 px-6 py-16">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="text-center mb-12">
          <h1 className="text-4xl md:text-5xl font-bold mb-4 text-slate-900 dark:text-white">
            🎭 Roles
          </h1>
          <p className="text-lg text-slate-700 dark:text-slate-300 max-w-3xl mx-auto">
            16 specialized personas spanning FinOps, AI, Security, Research, and more. Each role brings
            domain expertise, professional perspective, and context-aware guidance.
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
                {items.map((role) => (
                  <GlassCard key={role.id} className="p-6 hover:scale-105 transition-transform">
                    <div className="flex items-start gap-3 mb-3">
                      <div className="text-3xl">{role.emoji}</div>
                      <div className="flex-1">
                        <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-1">
                          {role.name}
                        </h3>
                        <p className="text-sm text-slate-600 dark:text-slate-400">
                          {role.description}
                        </p>
                      </div>
                    </div>
                    {role.personality?.motto && (
                      <div className="mt-3 pt-3 border-t border-slate-200 dark:border-slate-700">
                        <p className="text-xs italic text-slate-600 dark:text-slate-400">
                          "{role.personality.motto}"
                        </p>
                      </div>
                    )}
                    {role.personality?.principles && role.personality.principles.length > 0 && (
                      <div className="mt-3 pt-3 border-t border-slate-200 dark:border-slate-700">
                        <span className="text-xs font-semibold text-blue-600 dark:text-blue-400">
                          {role.personality.principles.length} core principles
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
