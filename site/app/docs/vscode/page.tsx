export default function VsCodeIntegrationPage() {
  return (
    <main className="min-h-screen bg-white dark:bg-slate-950 text-slate-900 dark:text-slate-50">
      <div className="max-w-4xl mx-auto px-4 py-12">
        {/* Breadcrumbs */}
        <nav className="mb-8 text-sm text-slate-600 dark:text-slate-400">
          <a href="/" className="hover:text-slate-900 dark:hover:text-slate-200">Home</a>
          <span className="mx-2">/</span>
          <a href="/" className="hover:text-slate-900 dark:hover:text-slate-200">Docs</a>
          <span className="mx-2">/</span>
          <span>VS Code</span>
        </nav>

        {/* Page Header */}
        <h1 className="text-4xl font-bold mb-2">VS Code + Maenifold</h1>
        <p className="text-lg text-slate-600 dark:text-slate-400 mb-10">
          Graph-aware software engineering agents inside VS Code, powered by Maenifold&apos;s knowledge graph.
        </p>

        {/* Overview */}
        <section className="mb-10">
          <h2 className="text-2xl font-bold mb-4">Overview</h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            The VS Code integration exposes two Maenifold-backed agents to your editor:
          </p>
          <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
            <li><strong>maenifold</strong> – a knowledge-graph enhanced SWE agent with persistent memory</li>
            <li><strong>agent-boss</strong> – an orchestration agent that decomposes work and coordinates subagents</li>
          </ul>
          <p className="text-slate-700 dark:text-slate-300 mt-4">
            Both agents use Maenifold&apos;s <code className="text-blue-600 dark:text-blue-400">memory://</code> corpus, concept graph,
            and MCP tools instead of relying on a single prompt or transient context.
          </p>
        </section>

        {/* Agents */}
        <section className="mb-10">
          <h2 className="text-2xl font-bold mb-4">Agents</h2>

          <div className="mb-6 p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <h3 className="text-xl font-bold mb-2">maenifold (SWE Agent)</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-3">
              The <code>maenifold</code> agent behaves like a regular coding assistant, but with access to Maenifold&apos;s
              knowledge graph and memory corpus.
            </p>
            <ul className="list-disc list-inside space-y-1 text-slate-700 dark:text-slate-300 ml-4">
              <li>On session start: runs sync, inspects recent activity, and builds graph context</li>
              <li>For exploratory questions: hypothesizes an answer with [[concepts]] and then searches those concepts</li>
              <li>For complex work: uses SequentialThinking to break down, revise, and persist reasoning</li>
            </ul>
          </div>

          <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <h3 className="text-xl font-bold mb-2">agent-boss (Orchestrator)</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-3">
              The <code>agent-boss</code> agent does not implement tasks directly. Instead, it decomposes work and delegates
              to other agents (usually <code>maenifold</code>) via subagent calls.
            </p>
            <ul className="list-disc list-inside space-y-1 text-slate-700 dark:text-slate-300 ml-4">
              <li>Breaks goals into atomic, testable tasks</li>
              <li>Launches parallel subagents and coordinates their work</li>
              <li>Uses the graph to aggregate and verify results before declaring success</li>
            </ul>
          </div>
        </section>

        {/* RAG Patterns */}
        <section className="mb-10">
          <h2 className="text-2xl font-bold mb-4">RAG Patterns</h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Both agents are concrete instances of the search and RAG patterns described in
            <a href="/docs/architecture" className="text-blue-600 dark:text-blue-400 ml-1 underline">Architecture</a>
            and
            <a href="/docs/claude-code" className="text-blue-600 dark:text-blue-400 ml-1 underline">Claude Code</a> docs.
          </p>
          <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
              <li><strong>Graph-RAG</strong> – all retrieval uses SearchMemories, BuildContext, and FindSimilarConcepts over the concept graph.</li>
              <li><strong>HYDE</strong> – for exploratory questions, the agent synthesizes a hypothetical answer with [[concepts]] and then searches those concepts.</li>
              <li><strong>FLARE-style startup</strong> – each session begins by proactively querying the graph (sync + recent activity + context building).</li>
              <li><strong>Self-RAG / CRAG</strong> – SequentialThinking provides the loop for revision, corrective retrieval, and assumption management.</li>
            </ul>
          </div>
        </section>

        {/* Configuration Pointer */}
        <section className="mb-10">
          <h2 className="text-2xl font-bold mb-4">Configuration</h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            The exact configuration depends on which VS Code extension you use for chat/agents (GitHub Copilot, Continue,
            etc.). In all cases, you will:
          </p>
          <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
            <li>Register the <code>maenifold</code> and <code>agent-boss</code> agent definitions from <code>docs/integrations/vscode/</code></li>
            <li>Ensure the Maenifold MCP server is available so tools like <code>maenifold/*</code> can be called</li>
            <li>Point the integration at your Maenifold workspace and <code>memory/</code> tree</li>
          </ul>
          <p className="text-slate-700 dark:text-slate-300 mt-4">
            See <code>docs/integrations/vscode/README.md</code> in the repository for detailed, tool-specific setup guidance.
          </p>
        </section>

        {/* See Also */}
        <section className="mb-4">
          <h2 className="text-2xl font-bold mb-4">See Also</h2>
          <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
            <li><a href="/docs/architecture" className="text-blue-600 dark:text-blue-400 underline">Architecture</a> – cognitive stack and graph layer</li>
            <li><a href="/docs/claude-code" className="text-blue-600 dark:text-blue-400 underline">Claude Code</a> – session-start FLARE integration</li>
          </ul>
        </section>
      </div>
    </main>
  );
}
