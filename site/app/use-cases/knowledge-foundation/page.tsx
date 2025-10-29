import Link from 'next/link';

export default function KnowledgeFoundationPage() {
  return (
    <div className="max-w-4xl mx-auto px-4 py-12">
      {/* Hero Section */}
      <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">
        Knowledge Building: Research Workflows + Obsidian Navigation
      </h1>
      <p className="text-lg text-slate-600 dark:text-slate-300 mb-12">
        Use AI research workflows to build rich knowledge graphs, then navigate visually in Obsidian
      </p>

      {/* Prerequisites */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Prerequisites</h2>
        <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300">
          <li>Claude Code installed (or compatible MCP client)</li>
          <li>Obsidian installed (optional, for visual navigation)</li>
          <li>Topic to research (e.g., FinOps Framework, system architecture)</li>
        </ul>
      </section>

      {/* Setup Steps */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Setup</h2>
        <div className="space-y-4">
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">1.</span>
            <div className="flex-1">
              <p className="text-slate-700 dark:text-slate-300 mb-2">Install maenifold:</p>
              <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
                <code className="text-sm font-mono text-slate-900 dark:text-slate-100">npm install -g @ma-collective/maenifold</code>
              </div>
            </div>
          </div>
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">2.</span>
            <p className="text-slate-700 dark:text-slate-300">Configure Claude Code with maenifold MCP server (see <Link href="/start" className="text-blue-600 dark:text-blue-400 hover:underline">Quick Start</Link>)</p>
          </div>
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">3.</span>
            <p className="text-slate-700 dark:text-slate-300">Optional: Set <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded">MAENIFOLD_ROOT</code> to Obsidian vault for visual navigation</p>
          </div>
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">4.</span>
            <p className="text-slate-700 dark:text-slate-300">Verify with: "Use agentic-research workflow to research [topic]"</p>
          </div>
        </div>
      </section>

      {/* Walkthrough Examples */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Walkthrough Examples</h2>

        {/* Example 1: Single-Agent Research */}
        <div className="mb-12">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 1: Single-Agent Deep Research (agentic-research)</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-4">
              <strong>Scenario:</strong> Build comprehensive knowledge about FinOps Framework to prepare for UC2.
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
            <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
              "Use agentic-research workflow to research the FinOps Framework"
            </p>

            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>What AI Does</strong> (11-step workflow):</p>
            <ol className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-decimal list-inside mb-4">
              <li><strong>Research Initiation:</strong> Creates research plan with <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[coverage-vector]]</code></li>
              <li><strong>Knowledge Baseline:</strong> Searches memory for existing <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[FinOps]]</code> knowledge</li>
              <li><strong>HyDE Query Expansion:</strong> Generates hypothetical documents for semantic search</li>
              <li><strong>Information Gathering:</strong> Executes web research, creates <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[cost-optimization]]</code> <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[commitment-strategy]]</code> concepts</li>
              <li><strong>Topic Discovery:</strong> Identifies emergent themes like <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[rate-optimization]]</code></li>
              <li><strong>Multi-Perspective Analysis:</strong> Analyzes from analyst, practitioner, CFO viewpoints</li>
              <li><strong>Synthesis & Reflexion:</strong> Six thinking hats quality review</li>
              <li><strong>Knowledge Integration:</strong> Creates memory://research/finops-framework/ with [[WikiLinks]]</li>
              <li><strong>Information Gain:</strong> Validates 20%+ new knowledge gained</li>
              <li><strong>Research Deliverable:</strong> Comprehensive report with findings</li>
              <li><strong>Quality Validation:</strong> Scores 7/10+ research quality</li>
            </ol>

            <div className="border-t border-slate-300 dark:border-slate-600 pt-4 mt-4">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Output:</strong></p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside mb-3">
                <li>10+ memory files at memory://research/finops-framework/</li>
                <li>50+ <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[concepts]]</code> in knowledge graph</li>
                <li>Ready for UC2 domain extensibility</li>
              </ul>
              <p className="text-sm text-blue-600 dark:text-blue-400">
                <strong>Why This Matters:</strong> You've seeded the graph with domain knowledge that AI will use to create custom roles.
              </p>
            </div>
          </div>
        </div>

        {/* Example 2: Multi-Agent Think Tank */}
        <div className="mb-12">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 2: Multi-Agent Think Tank (think-tank)</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-4">
              <strong>Scenario:</strong> Complex research requiring parallel agent investigation.
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
            <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
              "Use think-tank workflow to research cloud financial optimization"
            </p>

            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>What AI Does</strong> (4 waves, 15-20 agents):</p>

            <div className="space-y-4">
              <div>
                <p className="text-sm font-semibold text-slate-900 dark:text-white mb-2">Wave 1: Domain Scoping (4 parallel agents)</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Agent 1: Maps <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[knowledge-landscape]]</code> and <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[domains]]</code></li>
                  <li>Agent 2: Scans <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[academic-literature]]</code> and <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[seminal-works]]</code></li>
                  <li>Agent 3: Identifies <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[emerging-patterns]]</code> and <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[trends]]</code></li>
                  <li>Agent 4: Finds <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[knowledge-gaps]]</code> and <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[opportunities]]</code></li>
                </ul>
              </div>

              <div>
                <p className="text-sm font-semibold text-slate-900 dark:text-white mb-2">Wave 2: Deep Dive (3-5 parallel agents per domain)</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Each uses full agentic-research workflow</li>
                  <li>20+ [[WikiLinks]] per domain</li>
                  <li>Achieves 30%+ information gain</li>
                </ul>
              </div>

              <div>
                <p className="text-sm font-semibold text-slate-900 dark:text-white mb-2">Wave 3: Cross-Domain Synthesis (4 parallel agents)</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Finds <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[interdisciplinary-connections]]</code></li>
                  <li>Identifies <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[emergent-patterns]]</code></li>
                  <li>Reconciles contradictions</li>
                  <li>Generates <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[novel-insights]]</code></li>
                </ul>
              </div>

              <div>
                <p className="text-sm font-semibold text-slate-900 dark:text-white mb-2">Wave 4: Peer Review (4 parallel agents)</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Validates methodology (7/10+ score)</li>
                  <li>Assesses evidence quality</li>
                  <li>Verifies logic chains</li>
                  <li>Confirms 80%+ coverage</li>
                </ul>
              </div>
            </div>

            <div className="border-t border-slate-300 dark:border-slate-600 pt-4 mt-4">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Output:</strong></p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside mb-3">
                <li>40+ memory files with comprehensive domain knowledge</li>
                <li>100+ <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[concepts]]</code> in interconnected graph</li>
                <li>Multi-perspective synthesis ready for domain work</li>
              </ul>
              <p className="text-sm text-blue-600 dark:text-blue-400">
                <strong>Why This Matters:</strong> Parallel research accelerates knowledge building for complex domains.
              </p>
            </div>
          </div>
        </div>

        {/* Example 3: Obsidian Visual Navigation */}
        <div className="mb-12">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 3: Obsidian Visual Navigation</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-4">
              <strong>Scenario:</strong> After research, navigate your knowledge graph visually.
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Steps:</strong></p>
            <ol className="text-sm text-slate-700 dark:text-slate-300 space-y-2 list-decimal list-inside mb-4">
              <li>Open memory folder as Obsidian vault</li>
              <li>View graph visualization: <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[FinOps-Framework]]</code> connects to <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[cost-optimization]]</code>, <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[commitment-strategy]]</code>, <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[ROI]]</code></li>
              <li>Click [[WikiLinks]] to navigate between research files</li>
              <li>Use backlinks panel to see all mentions of <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[Azure]]</code></li>
              <li>Search across entire research corpus instantly</li>
            </ol>

            <div className="border-t border-slate-300 dark:border-slate-600 pt-4 mt-4 mb-4">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Why Obsidian?</strong></p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside">
                <li>Visual graph view shows concept relationships</li>
                <li>Bidirectional links surface unexpected connections</li>
                <li>Markdown-native, no lock-in</li>
                <li>Works alongside maenifold seamlessly</li>
              </ul>
            </div>

            <p className="text-sm text-blue-600 dark:text-blue-400">
              <strong>Why This Matters:</strong> Visual navigation accelerates understanding without leaving your workspace.
            </p>
          </div>
        </div>
      </section>

      {/* Built-In Workflows */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Built-In Workflows</h2>
        <p className="text-slate-700 dark:text-slate-300 mb-6">
          Two research workflows bundled with maenifold:
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">agentic-research</h3>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3">
              Single-agent, 11 steps, HyDE-enhanced
            </p>
            <code className="text-xs font-mono text-slate-600 dark:text-slate-400">/assets/workflows/agentic-research.json</code>
          </div>
          <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">think-tank</h3>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3">
              Multi-agent waves, 15-20 agents, parallel execution
            </p>
            <code className="text-xs font-mono text-slate-600 dark:text-slate-400">/assets/workflows/think-tank.json</code>
          </div>
        </div>
      </section>

      {/* Code Sample */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Code Sample</h2>
        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
          <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
            <code>{`# Single-agent research via CLI
maenifold --tool Workflow --payload '{
  "workflowId": "agentic-research",
  "response": "Research the FinOps Framework"
}'

# Multi-agent research (requires Claude Code with agent orchestration)
# Use think-tank workflow via Claude Code

# Optional: Visual navigation with Obsidian
export MAENIFOLD_ROOT="~/Documents/ObsidianVault"`}</code>
          </pre>
        </div>
      </section>

      {/* Common Pitfalls */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Common Pitfalls</h2>
        <div className="space-y-3 text-slate-700 dark:text-slate-300">
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Workflow choice:</strong> Use <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-sm">agentic-research</code> for focused research, <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-sm">think-tank</code> for complex multi-domain topics
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Concept density:</strong> Aim for 10-50+ [[WikiLinks]] per research file for rich graph connections
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Sync after research:</strong> Run <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-sm">Sync</code> tool to update graph database after creating memories
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Obsidian optional:</strong> Graph works without Obsidian—it just provides visual navigation layer
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>WikiLink syntax:</strong> Use <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-sm">[[concept-name]]</code> with hyphens for multi-word concepts, not spaces
            </div>
          </div>
        </div>
      </section>

      {/* Next Steps */}
      <section>
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Next Steps</h2>
        <ul className="space-y-3">
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/use-cases/domain-extensibility" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                UC2: Domain Extensibility
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Use this knowledge foundation to create domain-specific roles and workflows
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Try BuildContext
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Explore concept relationships on [[FinOps-Framework]]
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Use Visualize
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Generate Mermaid diagrams of knowledge architecture
              </p>
            </div>
          </li>
        </ul>
      </section>
    </div>
  );
}
