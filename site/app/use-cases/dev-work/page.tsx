import Link from 'next/link';

export default function DevWorkPage() {
  return (
    <div className="max-w-4xl mx-auto px-4 py-12">
      {/* Hero Section */}
      <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">
        Codex + maenifold: Persistent Development Context
      </h1>
      <p className="text-lg text-slate-600 dark:text-slate-300 mb-12">
        Never lose context between coding sessions. Your decisions, architecture, and progress persist in a living knowledge graph.
      </p>

      {/* Prerequisites */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Prerequisites</h2>
        <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300">
          <li>Codex installed</li>
          <li>Active development project</li>
          <li>Basic understanding of project documentation</li>
        </ul>
      </section>

      {/* Graph-First Reset Protocol */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Graph-First Reset Protocol</h2>
        <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800 mb-6">
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Because Codex (and all agents) experience memory resets between sessions, maenifold provides a systematic recovery protocol:
          </p>
          <ol className="list-decimal list-inside space-y-2 text-slate-700 dark:text-slate-300">
            <li><strong>Sync</strong> → Ensure graph reflects latest markdown changes</li>
            <li><strong>RecentActivity</strong> → Identify active sessions and recent documents</li>
            <li><strong>SearchMemories</strong> → Find relevant knowledge using hybrid search</li>
            <li><strong>BuildContext</strong> → Navigate concept relationships for full context</li>
          </ol>
          <p className="text-slate-600 dark:text-slate-400 mt-4 text-sm">
            This protocol ensures every session starts with complete context from the knowledge graph, not from scratch.
          </p>
        </div>

        {/* Core Workflows */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Plan Mode</h3>
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">Discovery and planning</p>
            <div className="text-sm font-mono text-slate-700 dark:text-slate-300 space-y-1">
              <div>Start → Sync →</div>
              <div>RecentActivity →</div>
              <div>SearchMemories →</div>
              <div>BuildContext/Visualize →</div>
              <div>Read files → Create plan</div>
            </div>
          </div>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Act Mode</h3>
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">Implementation and execution</p>
            <div className="text-sm font-mono text-slate-700 dark:text-slate-300 space-y-1">
              <div>Start → Resume session →</div>
              <div>Execute work →</div>
              <div>WriteMemory/EditMemory →</div>
              <div>Sync → Update context →</div>
              <div>Summarize results</div>
            </div>
          </div>
        </div>
        <p className="text-sm text-slate-600 dark:text-slate-400 mt-4">
          These workflows are embedded in the <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded">swe.md</code> agent instructions and automatically guide Codex through systematic development.
        </p>
      </section>

      {/* Setup Steps */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Setup</h2>
        <div className="space-y-6">
          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">1. Install maenifold</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>npm install -g @ma-collective/maenifold</code>
              </pre>
            </div>
          </div>

          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">2. Configure Codex</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-3">
              Add to <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded">~/.codex/config.toml</code>:
            </p>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`[mcp_servers.maenifold]
type = "stdio"
command = "maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600
env = { MAENIFOLD_ROOT = "~/maenifold" }`}</code>
              </pre>
            </div>
          </div>

          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">3. Optional: Add swe.md agent instructions</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-3">
              Copy agent instructions to <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded">~/.codex/agents/swe.md</code> for automatic graph-first protocol.
            </p>
          </div>

          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">4. Verify installation</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>codex chat &quot;List maenifold MCP tools&quot;</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Walkthrough Examples */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Walkthrough Examples</h2>

        {/* Example 1 */}
        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 1: Day 1 - Starting New Feature</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700 mb-4">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-3"><strong>Scenario:</strong> You're starting work on a new authentication feature.</p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>Codex Session:</strong></p>
            <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
              "I'm starting work on JWT authentication. Write a project brief and initial architecture decisions to memory."
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>What maenifold Does:</strong></p>
            <ol className="text-sm text-slate-700 dark:text-slate-300 space-y-2 list-decimal list-inside">
              <li>Creates <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">memory://projects/myapp/projectbrief.md</code>
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside text-xs">
                  <li>Scope: JWT authentication feature</li>
                  <li>Stakeholders: Backend team</li>
                  <li>Constraints: Must support refresh tokens</li>
                </ul>
              </li>
              <li>Creates <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">memory://projects/myapp/systemPatterns.md</code>
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside text-xs">
                  <li>Architecture: JWT stored in httpOnly cookies</li>
                  <li>Security: HMAC SHA-256 signing</li>
                  <li>Refresh strategy: Sliding window pattern</li>
                </ul>
              </li>
              <li>Creates <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">memory://projects/myapp/activeContext.md</code>
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside text-xs">
                  <li>Current: Implementing token generation</li>
                  <li>Next: Add refresh token rotation</li>
                  <li>Blockers: Need to decide token expiry time</li>
                </ul>
              </li>
            </ol>
            <p className="text-xs text-slate-600 dark:text-slate-400 mt-3">
              All files contain <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[JWT]]</code>, <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[authentication]]</code>, <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[security]]</code> WikiLinks.
            </p>
          </div>
        </div>

        {/* Example 2 */}
        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 2: Day 2 - Resume Work</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-3"><strong>Scenario:</strong> You're back the next day. Codex memory is reset.</p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>Codex Session:</strong></p>
            <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
              "What was I working on yesterday? Show recent activity."
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>maenifold Response via RecentActivity:</strong></p>
            <div className="bg-slate-200 dark:bg-slate-800 rounded p-3 text-xs font-mono text-slate-900 dark:text-slate-100 space-y-2 mb-4">
              <div>Recent activity (last 24 hours):</div>
              <div>1. memory://projects/myapp/activeContext.md (8 hours ago)</div>
              <div className="ml-4">- Current: Implementing token generation</div>
              <div className="ml-4">- Next: Add refresh token rotation</div>
              <div>2. memory://projects/myapp/systemPatterns.md (8 hours ago)</div>
              <div className="ml-4">- JWT architecture decisions documented</div>
              <div>3. memory://thinking/session-1234567890.md (8 hours ago)</div>
              <div className="ml-4">- Sequential thinking: Security considerations</div>
            </div>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Codex automatically:</strong></p>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside">
              <li>Reads activeContext.md</li>
              <li>Has full context of yesterday's decisions</li>
              <li>Knows exactly where to continue</li>
              <li>No need to re-explain the feature</li>
            </ul>
          </div>
        </div>

        {/* Example 3 */}
        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 3: Multi-Day Feature Development</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <div className="space-y-4 text-sm text-slate-700 dark:text-slate-300">
              <div>
                <strong>Day 1:</strong> Architecture & planning
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside">
                  <li>Write projectbrief.md, systemPatterns.md</li>
                  <li>Document <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[security-requirements]]</code></li>
                </ul>
              </div>
              <div>
                <strong>Day 2:</strong> Implementation starts
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside">
                  <li>Update activeContext.md with progress</li>
                  <li>Write memories about <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[implementation-challenges]]</code></li>
                  <li>Link to relevant <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[code-patterns]]</code></li>
                </ul>
              </div>
              <div>
                <strong>Day 3:</strong> Testing & refinement
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside">
                  <li>SearchMemories for <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[security-requirements]]</code></li>
                  <li>Verify all requirements met</li>
                  <li>Update progress.md with completion status</li>
                </ul>
              </div>
              <div>
                <strong>Day 4:</strong> Code review prep
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside">
                  <li>BuildContext around <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">[[JWT]]</code> concept</li>
                  <li>Generate summary of all related decisions</li>
                  <li>Create review checklist from memory</li>
                </ul>
              </div>
              <div className="pt-3 border-t border-slate-300 dark:border-slate-600">
                <strong>Result:</strong> Complete development history, every decision documented, full traceability from requirements to implementation.
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Project Memory Structure */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Project Memory Structure</h2>
        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
          <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
            <code>{`memory://
└── projects/
    └── myapp/
        ├── projectbrief.md        # Foundation: scope, stakeholders, constraints
        ├── productContext.md      # User needs, desired outcomes
        ├── systemPatterns.md      # Architecture, integrations, invariants
        ├── techContext.md         # Tooling, environment, dependencies
        ├── activeContext.md       # Current priorities, next steps, blockers
        └── progress.md            # Status log, milestone tracking`}</code>
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
              <strong>Forgetting to update activeContext:</strong> Update after each session or you'll lose "current state"
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Not using [[concepts]]:</strong> WikiLinks are how maenifold links knowledge—use liberally!
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Skipping Sync:</strong> Run Sync after writing memories so they're searchable
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Over-documenting:</strong> Focus on decisions and "why", not every code change
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Ignoring Graph-First Reset Protocol:</strong> Always start sessions with Sync → RecentActivity → SearchMemories to rebuild full context
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Reading files blindly:</strong> Use SearchMemories and BuildContext to navigate the graph, don't manually read every file
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Not tracking assumptions:</strong> Use Assumption Ledger for architectural decisions with uncertainty—link to activeContext/progress for traceability
            </div>
          </div>
        </div>
      </section>

      {/* Advanced Patterns */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Advanced Patterns</h2>

        <div className="space-y-6">
          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Pattern 1: Assumption Tracking</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`# When making architectural decisions with uncertainty
codex chat "We're assuming users want single sign-on. Track this assumption."

# maenifold creates assumption ledger entry
# Later, when assumption is validated/invalidated:
codex chat "Update assumption: SSO requirement confirmed by product team"`}</code>
              </pre>
            </div>
          </div>

          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Pattern 2: Cross-Project Learning</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`# Reuse patterns from previous projects
codex chat "Search memories for [[authentication]] patterns across all projects"

# maenifold returns authentication approaches from ALL projects
# Codex suggests best practices based on past experience`}</code>
              </pre>
            </div>
          </div>

          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Pattern 3: Team Knowledge Sharing</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`# Multiple developers sharing same MAENIFOLD_ROOT
export MAENIFOLD_ROOT="/shared/team-knowledge"

# Each developer's Codex writes to shared graph
# All team members benefit from collective knowledge
# Search finds insights from entire team's work`}</code>
              </pre>
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
              <Link href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Explore Tools
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Browse all maenifold tools including SequentialThinking
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/workflows" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Try Workflows
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Discover systematic development methodologies
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/use-cases/product-team" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Multi-Agent Orchestration
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                See team simulation with parallel agent waves
              </p>
            </div>
          </li>
        </ul>
      </section>
    </div>
  );
}
