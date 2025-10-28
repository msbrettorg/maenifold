export default function QuickStart() {
  return (
    <main className="min-h-screen bg-white dark:bg-slate-950">
      <div className="max-w-4xl mx-auto px-6 py-12">
        {/* Quick Start Header */}
        <h1 className="text-4xl font-bold mb-12 text-slate-900 dark:text-white">
          Quick start
        </h1>

        {/* Install Section */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">
            Install
          </h2>
          <pre className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 overflow-x-auto border border-slate-200 dark:border-slate-800">
            <code className="text-sm text-slate-900 dark:text-slate-100 font-mono">
              npm install -g @ma-collective/maenifold
            </code>
          </pre>
        </section>

        {/* MCP Interface Section */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">
            MCP Interface
          </h2>

          <div className="mb-8">
            <p className="text-slate-700 dark:text-slate-300 mb-4 font-semibold">
              Claude Code, Continue, Cline - Add to MCP config:
            </p>
            <pre className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 overflow-x-auto border border-slate-200 dark:border-slate-800">
              <code className="text-sm text-slate-900 dark:text-slate-100 font-mono">
{`{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": {"MAENIFOLD_ROOT": "~/maenifold"}
    }
  }
}`}
              </code>
            </pre>
          </div>

          <div className="mb-8">
            <p className="text-slate-700 dark:text-slate-300 mb-4 font-semibold">
              Codex - Add to <code className="bg-slate-100 dark:bg-slate-900 px-2 py-1 rounded text-slate-900 dark:text-slate-100">~/.codex/config.toml</code>:
            </p>
            <pre className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 overflow-x-auto border border-slate-200 dark:border-slate-800">
              <code className="text-sm text-slate-900 dark:text-slate-100 font-mono">
{`[mcp_servers.maenifold]
type = "stdio"
command = "maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600
env = { MAENIFOLD_ROOT = "~/maenifold" }`}
              </code>
            </pre>
          </div>

          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Try it: <code className="bg-slate-100 dark:bg-slate-900 px-2 py-1 rounded text-slate-900 dark:text-slate-100">"Write a memory about our architecture decisions"</code>
          </p>
        </section>

        {/* CLI Interface Section */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">
            CLI Interface
          </h2>

          <p className="text-slate-700 dark:text-slate-300 mb-6">
            Use maenifold directly in scripts, pipelines, or with non-MCP clients:
          </p>

          <div className="space-y-6">
            {/* WriteMemory Example */}
            <div>
              <pre className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 overflow-x-auto border border-slate-200 dark:border-slate-800">
                <code className="text-sm text-slate-900 dark:text-slate-100 font-mono">
{`# Write a memory with WikiLinks
maenifold --tool WriteMemory --payload '{
  "title": "Architecture Decisions",
  "content": "Our [[microservices]] use [[event-sourcing]] for [[audit-trails]]"
}'`}
                </code>
              </pre>
            </div>

            {/* SequentialThinking Example */}
            <div>
              <pre className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 overflow-x-auto border border-slate-200 dark:border-slate-800">
                <code className="text-sm text-slate-900 dark:text-slate-100 font-mono">
{`# Continue a sequential thinking session
maenifold --tool SequentialThinking --payload '{
  "sessionId": "session-1234567890",
  "response": "After analyzing the architecture...",
  "nextThoughtNeeded": true
}'`}
                </code>
              </pre>
            </div>

            {/* SearchMemories Example */}
            <div>
              <pre className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 overflow-x-auto border border-slate-200 dark:border-slate-800">
                <code className="text-sm text-slate-900 dark:text-slate-100 font-mono">
{`# Search memories with hybrid mode
maenifold --tool SearchMemories --payload '{
  "query": "authentication patterns",
  "mode": "Hybrid",
  "pageSize": 10
}'`}
                </code>
              </pre>
            </div>
          </div>
        </section>

        {/* Feature Parity Statement */}
        <section className="mb-12">
          <p className="text-slate-700 dark:text-slate-300 leading-relaxed">
            <strong>MCP and CLI have full feature parity.</strong> Start a session via MCP and continue it via CLI, or vice versa. The system supports concurrent agents using the same memory location - perfect for multi-agent pipelines or parallel workflows.
          </p>
        </section>
      </div>
    </main>
  );
}
