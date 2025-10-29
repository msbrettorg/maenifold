import Link from 'next/link';

export default function ProductTeamPage() {
  return (
    <div className="max-w-4xl mx-auto px-4 py-12">
      {/* Hero Section */}
      <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">
        Claude Code + Subagents: Orchestrate an Entire Product Team
      </h1>
      <p className="text-lg text-slate-600 dark:text-slate-300 mb-12">
        Deploy 25+ agents in parallel waves, simulating PM, engineers, QA, and red team—all sharing a single knowledge graph.
      </p>

      {/* The Hero Demo */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">
          The Hero Demo: 68 Minutes, 25 Agents, Real Production Work
        </h2>
        <p className="text-slate-700 dark:text-slate-300 mb-6">
          On January 21, 2025, we ran a comprehensive demonstration of multi-agent orchestration using maenifold. This wasn't a toy example—it was real production work that discovered and fixed critical bugs.
        </p>

        <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800 mb-6">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-4">Demo Stats</h3>
          <div className="grid grid-cols-2 gap-4 text-sm text-slate-700 dark:text-slate-300">
            <div><strong>Duration:</strong> 68 minutes total</div>
            <div><strong>Agents Deployed:</strong> 25 agents across 2 phases</div>
            <div><strong>Issues Fixed:</strong> 4 bugs/features</div>
            <div><strong>Test Coverage:</strong> 2,031 lines of tests added</div>
            <div className="col-span-2"><strong>Knowledge Graph Impact:</strong> 171,506 new concept relations</div>
          </div>
        </div>

        {/* Part 1: PM-lite */}
        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Part 1: PM-lite Protocol (Discovery & Testing)</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <div className="space-y-2 text-sm text-slate-700 dark:text-slate-300">
              <div><strong>Duration:</strong> 28 minutes</div>
              <div><strong>Agents:</strong> 12 agents across 4 waves</div>
              <ul className="ml-6 mt-2 space-y-1 list-disc list-inside text-xs">
                <li>Wave 1: 3 agents (core functionality)</li>
                <li>Wave 2: 4 agents (integration testing)</li>
                <li>Wave 3: 3 agents (edge cases)</li>
                <li>Wave 4: 2 agents (verification)</li>
              </ul>
              <div><strong>Coordination:</strong> Sequential thinking only (no rigid workflow)</div>
              <div><strong>Result:</strong> Discovered critical move operation bug, 85% test success rate</div>
            </div>
          </div>
        </div>

        {/* Part 2: Agentic-SLC */}
        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Part 2: Agentic-SLC Workflow (Issue Remediation)</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <div className="space-y-2 text-sm text-slate-700 dark:text-slate-300">
              <div><strong>Duration:</strong> 40 minutes</div>
              <div><strong>Agents:</strong> 13 agents total</div>
              <ul className="ml-6 mt-2 space-y-1 list-disc list-inside text-xs">
                <li>7 implementation agents across 3 waves</li>
                <li>6 discovery/validation agents</li>
              </ul>
              <div><strong>Coordination:</strong> 17-step workflow with quality gates</div>
              <div><strong>Result:</strong> Fixed 3 issues, comprehensive regression tests added</div>
            </div>
          </div>
        </div>
      </section>

      {/* Prerequisites */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Prerequisites</h2>
        <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300">
          <li>Claude Code installed and configured</li>
          <li>Understanding of multi-agent patterns</li>
          <li>Optional: Read demo artifacts first for context</li>
        </ul>
      </section>

      {/* Setup */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Setup for Multi-Agent Orchestration</h2>

        <div className="space-y-6">
          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">1. Single Claude Code instance as orchestrator</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`# Set shared knowledge location
export MAENIFOLD_ROOT="~/project-sprint"

# Start Claude Code
claude-code`}</code>
              </pre>
            </div>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">2. Orchestrator creates plan</h3>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2">
              "Create a sequential thinking session for orchestrating test coverage. Plan agent waves: discovery → implementation → verification → red team."
            </p>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">3. Deploy agent waves (using Claude Code subagents)</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`"Launch 4 parallel subagents:
- TST-001: Test API endpoints
- TST-002: Test CLI parity
- TST-003: Test error handling
- TST-004: Performance benchmarks

Each agent should write findings to memory with [[test-result]] tags."`}</code>
              </pre>
            </div>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">4. Agents work in parallel, writing to shared graph</h3>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-2 list-disc list-inside">
              <li>Each agent has own sequential thinking session</li>
              <li>All write to same memory:// location</li>
              <li>Knowledge graph automatically links related concepts</li>
              <li>Orchestrator monitors progress via RecentActivity</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">5. Orchestrator synthesizes results</h3>
            <p className="text-sm text-slate-700 dark:text-slate-300">
              "Search memories for [[test-result]] from the last 10 minutes. Build context and create summary report."
            </p>
          </div>
        </div>
      </section>

      {/* Simplified Walkthrough */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Walkthrough: Simplified PM-lite Demo</h2>
        <p className="text-slate-700 dark:text-slate-300 mb-6">
          Let's recreate a scaled-down version of the demo (4 agents instead of 12):
        </p>

        <div className="space-y-8">
          {/* Step 1 */}
          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Step 1: Orchestrator Creates Plan</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt</strong> (you, as PM):</p>
              <p className="text-sm font-mono text-slate-900 dark:text-slate-100">
                "I need to test our search functionality. Create a plan for 4 test agents: Agent A (hybrid search), Agent B (semantic search), Agent C (full-text search), Agent D (edge cases). Create a sequential thinking session to track orchestration."
              </p>
            </div>
          </div>

          {/* Step 2 */}
          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Step 2: Launch Wave 1 (Parallel Testing)</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
              <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
                "Launch 4 subagents in parallel. Each should: 1) Run assigned tests, 2) Write results to memory with [[test-result]] tag, 3) Document any bugs with [[bug]] tag. Use agent names: TST-A, TST-B, TST-C, TST-D"
              </p>
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>What Happens:</strong></p>
              <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside">
                <li>Claude Code spawns 4 parallel agents (via Task tool)</li>
                <li>Each agent runs independently</li>
                <li>Each writes to memory://testing/ folder</li>
                <li>All agents share the knowledge graph</li>
                <li>You see 4 progress indicators running simultaneously</li>
              </ul>
            </div>
          </div>

          {/* Step 3 */}
          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Step 3: Monitor Progress</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
              <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-3">
                "Check recent activity. Show me what each agent has completed."
              </p>
              <div className="bg-slate-200 dark:bg-slate-800 rounded p-3 text-xs font-mono text-slate-900 dark:text-slate-100">
                <div>Recent activity:</div>
                <div>- TST-A: Completed hybrid search tests (2 min ago) ✅</div>
                <div>- TST-B: Found edge case bug in semantic search (1 min ago) ⚠️</div>
                <div>- TST-C: Full-text tests passing (1 min ago) ✅</div>
                <div>- TST-D: Edge case testing in progress...</div>
              </div>
            </div>
          </div>

          {/* Step 4 */}
          <div>
            <h3 className="text-xl font-semibold mb-3 text-slate-900 dark:text-white">Step 4: Synthesize Results</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
              <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-3">
                "Search memories for [[test-result]] and [[bug]]. Build context and create summary report."
              </p>
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>maenifold Automatically:</strong></p>
              <ol className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-decimal list-inside">
                <li>Searches all agent memories</li>
                <li>Finds test results + bug reports</li>
                <li>Builds concept graph showing relationships</li>
                <li>Generates comprehensive report</li>
              </ol>
              <p className="text-sm text-slate-700 dark:text-slate-300 mt-3"><strong>Report Includes:</strong> Test coverage matrix, pass/fail summary, bug details with links to agent reports, recommendations for next steps</p>
            </div>
          </div>
        </div>
      </section>

      {/* Demo Artifacts */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Real Demo Artifacts</h2>
        <p className="text-slate-700 dark:text-slate-300 mb-6">
          We've preserved the complete demo artifacts showing every step:
        </p>

        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700 mb-6">
          <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>Location:</strong> <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded">/Users/brett/src/ma-collective/maenifold/assets/demo-artifacts</code></p>

          <div className="mb-4">
            <p className="text-sm font-semibold text-slate-900 dark:text-white mb-2">Part 1: PM-lite Protocol (part1-pm-lite/)</p>
            <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
              <li>orchestration-session.md: Real-time orchestration thoughts (session-1758470366887)</li>
              <li>test-matrix-orchestration-plan.md: 50+ test cases, 4-wave architecture</li>
              <li>E2E_TEST_REPORT.md: Final results showing 85% success rate</li>
              <li>demo-final-report.md: Comprehensive report from VRFY-02 agent</li>
            </ul>
          </div>

          <div>
            <p className="text-sm font-semibold text-slate-900 dark:text-white mb-2">Part 2: Agentic-SLC Workflow (part2-agentic-slc/)</p>
            <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
              <li>agentic-slc-thinking-session.md: Sprint progress (session-1758474798193)</li>
              <li>agentic-slc-workflow-session.md: Complete 17-step workflow (includes embedded RTM.md content)</li>
              <li>RTM.md: Requirements traceability matrix (27 atomic items) - embedded in workflow session, not standalone</li>
              <li>sprint-specifications.md: Detailed specs with line numbers</li>
              <li>implementation-plan.md: 7 agents, 3 waves, 22 tasks</li>
              <li>code-justification-report.md: Justification for every line of code</li>
              <li>sprint-retrospective.md: Learnings and metrics</li>
            </ul>
          </div>
        </div>

        <div className="bg-slate-200 dark:bg-slate-800 rounded p-4 text-sm font-mono text-slate-900 dark:text-slate-100">
          <div className="mb-2"># Browse the artifacts:</div>
          <div>cd /Users/brett/src/ma-collective/maenifold/assets/demo-artifacts</div>
          <div>cat README.md  # Start here for overview</div>
          <div>cd part1-pm-lite</div>
          <div>cat orchestration-session.md  # See real orchestration thoughts</div>
          <div className="mt-2 text-xs text-slate-600 dark:text-slate-400"># Note: RTM.md content is embedded in part2-agentic-slc/agentic-slc-workflow-session.md</div>
          <div className="text-xs text-slate-600 dark:text-slate-400"># Look for the RTM section within the workflow session file</div>
        </div>
      </section>

      {/* Key Patterns */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Key Patterns from Demo</h2>

        <div className="space-y-6">
          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Pattern 1: Adaptive Wave Deployment</h3>
            <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4 border border-blue-200 dark:border-blue-800">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-3">
                Instead of launching all 12 agents at once, the PM orchestrator:
              </p>
              <ol className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-decimal list-inside">
                <li>Launched Wave 1 (4 agents): Core functionality tests</li>
                <li>Monitored results via RecentActivity</li>
                <li>Discovered critical bug in Wave 1 results</li>
                <li>Adapted Wave 2 plan based on findings</li>
                <li>Launched Wave 2 (3 agents): Bug investigation + regression tests</li>
                <li>Continued adaptive deployment through Waves 3 & 4</li>
              </ol>
              <p className="text-sm text-blue-600 dark:text-blue-400 mt-3">
                <strong>Result:</strong> More efficient than rigid plan, found critical issues early
              </p>
            </div>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Pattern 2: Shared Sequential Thinking</h3>
            <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4 border border-blue-200 dark:border-blue-800">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2">
                All 12 agents contributed to SAME sequential thinking session:
              </p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside mb-3">
                <li>PM wrote orchestration thoughts</li>
                <li>Agents wrote progress updates</li>
                <li>Verification agents wrote validation results</li>
                <li>Red team wrote security concerns</li>
              </ul>
              <p className="text-sm text-blue-600 dark:text-blue-400">
                <strong>Result:</strong> Complete audit trail, single source of truth for entire sprint
              </p>
            </div>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Pattern 3: Knowledge Graph Synthesis</h3>
            <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4 border border-blue-200 dark:border-blue-800">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2">
                After all agents completed work:
              </p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside mb-3">
                <li>171,506 new concept relations added to graph</li>
                <li>BuildContext on [[test-coverage]] revealed connections across all agent work</li>
                <li>Visualize generated architecture diagrams from agent findings</li>
                <li>SearchMemories instantly retrieved related work across 25 agents</li>
              </ul>
              <p className="text-sm text-blue-600 dark:text-blue-400">
                <strong>Result:</strong> Knowledge persists beyond demo, available for future work
              </p>
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
              <p className="text-slate-700 dark:text-slate-300 font-semibold">Start with single-agent workflows</p>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Try <Link href="/use-cases/dev-work" className="text-blue-600 dark:text-blue-400 hover:underline">UC3</Link> first—you don't need multi-agent to benefit
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Explore Sequential Thinking
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Foundation for both single and multi-agent workflows
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <p className="text-slate-700 dark:text-slate-300 font-semibold">Browse demo artifacts</p>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                See real orchestration patterns and agent coordination strategies
              </p>
            </div>
          </li>
        </ul>
      </section>
    </div>
  );
}
