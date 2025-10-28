export default function QuickStartPage() {
  return (
    <div className="max-w-4xl mx-auto px-4 py-12">
      <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">Quick Start</h1>
      <p className="text-lg text-slate-600 dark:text-slate-300 mb-12">Get maenifold running in minutes.</p>

      {/* Installation Section */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Installation</h2>
        <p className="text-slate-700 dark:text-slate-300 mb-6">
          Install maenifold globally using npm:
        </p>
        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 mb-6 border border-slate-200 dark:border-slate-700">
          <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
            <code>npm install -g @ma-collective/maenifold</code>
          </pre>
        </div>
      </section>

      {/* MCP Configuration Section */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">MCP Interface</h2>

        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">
            Claude Code, Continue, Cline
          </h3>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Add maenifold to your MCP configuration:
          </p>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
              <code>{`{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": {"MAENIFOLD_ROOT": "~/maenifold"}
    }
  }
}`}</code>
            </pre>
          </div>
        </div>

        <div className="mb-8">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">
            Codex
          </h3>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Add to <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-slate-900 dark:text-slate-100">~/.codex/config.toml</code>:
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
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Try it out by asking: <em>"Write a memory about our architecture decisions"</em>
          </p>
        </div>
      </section>

      {/* CLI Interface Section */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">CLI Interface</h2>
        <p className="text-slate-700 dark:text-slate-300 mb-6">
          Use maenifold directly in scripts, pipelines, or with non-MCP clients:
        </p>

        <div className="space-y-8">
          {/* WriteMemory Example */}
          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">
              Write a memory with WikiLinks
            </h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`maenifold --tool WriteMemory --payload '{
  "title": "Architecture Decisions",
  "content": "Our [[microservices]] use [[event-sourcing]] for [[audit-trails]]"
}'`}</code>
              </pre>
            </div>
          </div>

          {/* SequentialThinking Example */}
          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">
              Continue a sequential thinking session
            </h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`maenifold --tool SequentialThinking --payload '{
  "sessionId": "session-1234567890",
  "response": "After analyzing the architecture...",
  "nextThoughtNeeded": true
}'`}</code>
              </pre>
            </div>
          </div>

          {/* SearchMemories Example */}
          <div>
            <h3 className="text-lg font-semibold mb-3 text-slate-900 dark:text-white">
              Search memories with hybrid mode
            </h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
              <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
                <code>{`maenifold --tool SearchMemories --payload '{
  "query": "authentication patterns",
  "mode": "Hybrid",
  "pageSize": 10
}'`}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Feature Parity Section */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Full Feature Parity</h2>
        <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800">
          <p className="text-slate-700 dark:text-slate-300">
            <strong>MCP and CLI have full feature parity.</strong> Start a session via MCP and continue it via CLI, or vice versa. The system supports concurrent agents using the same memory location — perfect for multi-agent pipelines or parallel workflows.
          </p>
        </div>
      </section>

      {/* Next Steps Section */}
      <section>
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Next Steps</h2>
        <ul className="space-y-4">
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <a href="/docs/architecture" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Learn the Cognitive Stack
              </a>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Understand the architecture behind maenifold's knowledge graph
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <a href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Explore Tools
              </a>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Browse all 26 available tools with complete documentation
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <a href="/workflows" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Discover Workflows
              </a>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Find the perfect workflow methodology for your use case
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <a href="/docs/philosophy" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Understand the Philosophy
              </a>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Learn the principles and design philosophy behind maenifold
              </p>
            </div>
          </li>
        </ul>
      </section>
    </div>
  )
}
