import { GlassCard } from '@/app/components/GlassCard';

export default function ClaudeCodeIntegrationPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900 px-6 py-16">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="text-center mb-12">
          <h1 className="text-4xl md:text-5xl font-bold mb-4 text-slate-900 dark:text-white">
            Claude Code Integration
          </h1>
          <p className="text-lg text-slate-700 dark:text-slate-300 max-w-3xl mx-auto">
            Automatic graph-based context restoration for every Claude Code session.
            The knowledge graph becomes your continuous context window.
          </p>
        </div>

        {/* Documentation Reference */}
        <GlassCard className="p-6 mb-8 bg-blue-50 dark:bg-slate-800 border-l-4 border-blue-500">
          <p className="text-sm text-slate-700 dark:text-slate-300">
            📚 <strong>Complete Documentation:</strong>{' '}
            <code className="bg-slate-200 dark:bg-slate-900 px-2 py-1 rounded text-sm">
              ~/maenifold/assets/integrations/claude-code/README.md
            </code>
          </p>
          <p className="text-xs text-slate-600 dark:text-slate-400 mt-2">
            For Claude Code hooks documentation, see{' '}
            <a
              href="https://docs.claude.com/en/docs/claude-code/hooks"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 dark:text-blue-400 hover:underline"
            >
              docs.claude.com/en/docs/claude-code/hooks
            </a>
          </p>
        </GlassCard>

        {/* What It Does */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            🧠 What It Does
          </h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Every Claude Code session automatically restores context using a FLARE-style proactive Graph-RAG pass:
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="flex items-start gap-3">
              <div className="text-2xl">📊</div>
              <div>
                <h3 className="font-semibold text-slate-900 dark:text-white">Queries Recent Activity</h3>
                <p className="text-sm text-slate-600 dark:text-slate-400">
                  Finds your latest work from the knowledge graph (RecentActivity)
                </p>
              </div>
            </div>
            <div className="flex items-start gap-3">
              <div className="text-2xl">🔍</div>
              <div>
                <h3 className="font-semibold text-slate-900 dark:text-white">Extracts Concepts</h3>
                <p className="text-sm text-slate-600 dark:text-slate-400">
                  Identifies top [[concepts]] from recent sessions
                </p>
              </div>
            </div>
            <div className="flex items-start gap-3">
              <div className="text-2xl">🔗</div>
              <div>
                <h3 className="font-semibold text-slate-900 dark:text-white">Builds Context</h3>
                <p className="text-sm text-slate-600 dark:text-slate-400">
                  Traverses the concept graph with BuildContext and semantic weights
                </p>
              </div>
            </div>
            <div className="flex items-start gap-3">
              <div className="text-2xl">💉</div>
              <div>
                <h3 className="font-semibold text-slate-900 dark:text-white">Injects Knowledge</h3>
                <p className="text-sm text-slate-600 dark:text-slate-400">
                  ~5K tokens of graph-derived context injected at session start
                </p>
              </div>
            </div>
          </div>
        </GlassCard>

        {/* RAG Patterns */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">🧩 RAG Patterns</h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            This integration is a concrete instance of the patterns described in the Maenifold search and scripting docs.
          </p>
          <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
            <li><strong>Graph-RAG:</strong> context comes from the concept graph (BuildContext + SearchMemories), not just flat file snippets.</li>
            <li><strong>FLARE-style startup:</strong> retrieval runs at session start, before your first prompt.</li>
            <li><strong>Multi-hop context:</strong> related concepts and files across 1–2 hops become part of the preamble.</li>
          </ul>
        </GlassCard>

        {/* How It Works */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            ⚙️ How It Works
          </h2>
          <div className="flex items-center justify-between text-center text-sm mb-6">
            <div className="flex-1">
              <div className="text-3xl mb-2">🚀</div>
              <div className="font-semibold text-slate-900 dark:text-white">Session Start</div>
            </div>
            <div className="text-slate-400">→</div>
            <div className="flex-1">
              <div className="text-3xl mb-2">📊</div>
              <div className="font-semibold text-slate-900 dark:text-white">Recent Activity</div>
            </div>
            <div className="text-slate-400">→</div>
            <div className="flex-1">
              <div className="text-3xl mb-2">🔍</div>
              <div className="font-semibold text-slate-900 dark:text-white">Extract Concepts</div>
            </div>
            <div className="text-slate-400">→</div>
            <div className="flex-1">
              <div className="text-3xl mb-2">🔎</div>
              <div className="font-semibold text-slate-900 dark:text-white">Query Graph</div>
            </div>
            <div className="text-slate-400">→</div>
            <div className="flex-1">
              <div className="text-3xl mb-2">🔗</div>
              <div className="font-semibold text-slate-900 dark:text-white">Build Context</div>
            </div>
            <div className="text-slate-400">→</div>
            <div className="flex-1">
              <div className="text-3xl mb-2">💉</div>
              <div className="font-semibold text-slate-900 dark:text-white">Inject Knowledge</div>
            </div>
          </div>
          <div className="bg-blue-50 dark:bg-slate-800 rounded-lg p-4 border-l-4 border-blue-500">
            <p className="text-sm text-slate-700 dark:text-slate-300 italic">
              "The graph is the context window" - Traditional: Files → Text → Context.
              Maenifold: Knowledge Graph → Semantic Relations → Understanding.
            </p>
          </div>
        </GlassCard>

        {/* Example Output */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            📝 Example Output
          </h2>
          <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-6 text-slate-300 font-mono text-sm">
            <div className="text-blue-400 font-bold mb-4">🧠 Knowledge Graph Context Restoration</div>

            <div className="mb-4">
              <div className="text-green-400 font-bold">### [[authentication]]</div>
              <div className="ml-4 text-slate-400">Context: JWT implementation with refresh tokens</div>
              <div className="ml-4 text-slate-400">Related: oauth (12 files), security (8 files)</div>
              <div className="ml-4 text-slate-500">Content: "Implementing RS256 token signing..."</div>
            </div>

            <div className="mb-4">
              <div className="text-green-400 font-bold">### [[database-migration]]</div>
              <div className="ml-4 text-slate-400">Context: Schema versioning strategy</div>
              <div className="ml-4 text-slate-400">Related: postgresql (15 files), migrations (10 files)</div>
              <div className="ml-4 text-slate-500">Content: "Using Flyway for version control..."</div>
            </div>

            <div className="text-slate-500 italic">[More concepts with actual context...]</div>
          </div>
        </GlassCard>

        {/* Setup */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            🔧 Setup
          </h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Configure Claude Code to restore context from your knowledge graph:
          </p>
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-slate-900 dark:text-white mb-2">1. Copy the hook</h3>
              <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-4">
                <code className="text-green-400 font-mono text-sm">
                  cp ~/maenifold/assets/integrations/claude-code/hooks/session_start.sh ~/.claude/hooks/<br />
                  chmod +x ~/.claude/hooks/session_start.sh
                </code>
              </div>
            </div>
            <div>
              <h3 className="font-semibold text-slate-900 dark:text-white mb-2">2. Update settings.json</h3>
              <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-4">
                <code className="text-blue-300 font-mono text-sm">
                  {'{'}<br />
                  &nbsp;&nbsp;"hooks": {'{'}<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;"SessionStart": [{'{'}<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;"matcher": "*",<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;"hooks": [{'{'}<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;"type": "command",<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;"command": "~/.claude/hooks/session_start.sh"<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;{'}'}]<br />
                  &nbsp;&nbsp;&nbsp;&nbsp;{'}'}]<br />
                  &nbsp;&nbsp;{'}'}<br />
                  {'}'}
                </code>
              </div>
            </div>
          </div>
        </GlassCard>

        {/* Configuration */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            ⚙️ Configuration
          </h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Edit <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-sm">~/.claude/hooks/session_start.sh</code> (top of file):
          </p>
          <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-4 mb-4">
            <code className="text-green-400 font-mono text-sm whitespace-pre">
{`GRAPH_DEPTH=2              # How many hops (1-3)
MAX_ENTITIES=10            # Related concepts per hop (3-20)
INCLUDE_CONTENT=false      # Show content previews? (true/false)
MAX_TOKENS=5000            # Approximate token budget
MAX_CONCEPTS=10            # Max top concepts to process`}
            </code>
          </div>
          <div className="bg-blue-50 dark:bg-slate-800 rounded-lg p-4">
            <p className="text-sm text-slate-700 dark:text-slate-300 font-semibold mb-2">
              Why defaults work:
            </p>
            <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1">
              <li>• <code className="bg-slate-200 dark:bg-slate-900 px-1 rounded">INCLUDE_CONTENT=false</code> shows more graph structure (20+ concepts vs 3)</li>
              <li>• <code className="bg-slate-200 dark:bg-slate-900 px-1 rounded">GRAPH_DEPTH=2</code> includes both direct and expanded relations</li>
              <li>• <code className="bg-slate-200 dark:bg-slate-900 px-1 rounded">MAX_ENTITIES=10</code> provides broad context without overwhelming</li>
            </ul>
          </div>
        </GlassCard>

        {/* Troubleshooting */}
        <GlassCard className="p-8 mb-8">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            🔍 Troubleshooting
          </h2>
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-slate-900 dark:text-white mb-2">Test the hook manually</h3>
              <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-4">
                <code className="text-green-400 font-mono text-sm">
                  echo '{`{"session_id":"test"}`}' | ~/.claude/hooks/session_start.sh
                </code>
              </div>
            </div>
            <div>
              <h3 className="font-semibold text-slate-900 dark:text-white mb-2">Check Maenifold status</h3>
              <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-4">
                <code className="text-green-400 font-mono text-sm">
                  maenifold --tool MemoryStatus
                </code>
              </div>
            </div>
            <div>
              <h3 className="font-semibold text-slate-900 dark:text-white mb-2">Verify hook is registered</h3>
              <div className="bg-slate-900 dark:bg-slate-950 rounded-lg p-4">
                <code className="text-green-400 font-mono text-sm">
                  cat ~/.claude/settings.json | jq '.hooks.SessionStart'
                </code>
              </div>
            </div>
          </div>
        </GlassCard>

        {/* Philosophy */}
        <div className="text-center bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg p-8 text-white">
          <p className="text-2xl font-bold">
            The graph is the context window
          </p>
        </div>
      </div>
    </div>
  );
}
