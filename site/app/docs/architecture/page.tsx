export default function ArchitecturePage() {
  return (
    <main className="min-h-screen bg-white dark:bg-slate-950 text-slate-900 dark:text-slate-50">
      <div className="max-w-4xl mx-auto px-4 py-12">
        {/* Breadcrumbs */}
        <nav className="mb-8 text-sm text-slate-600 dark:text-slate-400">
          <a href="/" className="hover:text-slate-900 dark:hover:text-slate-200">Home</a>
          <span className="mx-2">/</span>
          <a href="/" className="hover:text-slate-900 dark:hover:text-slate-200">Docs</a>
          <span className="mx-2">/</span>
          <span>Architecture</span>
        </nav>

        {/* Page Header */}
        <h1 className="text-4xl font-bold mb-2">Architecture</h1>
        <p className="text-lg text-slate-600 dark:text-slate-400 mb-12">
          Understanding the Cognitive Stack that powers Maenifold
        </p>

        {/* Cognitive Stack Overview */}
        <section className="mb-12">
          <h2 className="text-3xl font-bold mb-6">The Cognitive Stack</h2>
          <p className="text-slate-700 dark:text-slate-300 mb-6">
            Maenifold organizes intelligence into three interconnected layers, each serving a distinct purpose in the reasoning pipeline.
          </p>

          {/* Memory Layer */}
          <div className="mb-10 p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <h3 className="text-2xl font-bold mb-4">4.1 Memory Layer (<code className="text-blue-600 dark:text-blue-400">memory://</code>)</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-4">
              Every piece of knowledge lives as a markdown file with a unique URI:
            </p>
            <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
              <li><code className="text-blue-600 dark:text-blue-400">memory://decisions/api-design</code> - Architectural decisions</li>
              <li><code className="text-blue-600 dark:text-blue-400">memory://thinking/session-12345</code> - Sequential thinking sessions</li>
              <li><code className="text-blue-600 dark:text-blue-400">memory://research/rag-comparison</code> - Research notes</li>
            </ul>
            <p className="text-slate-700 dark:text-slate-300 mt-4">
              All files are human-readable, Obsidian-compatible, and persist across sessions.
            </p>
          </div>

          {/* Graph Layer */}
          <div className="mb-10 p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <h3 className="text-2xl font-bold mb-4">4.2 Graph Layer (SQLite + Vectors)</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-4">
              Automatic graph construction from WikiLinks with:
            </p>
            <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
              <li><strong>384-dimensional embeddings</strong> for semantic similarity</li>
              <li><strong>Edge weights</strong> that strengthen with repeated mentions</li>
              <li><strong>Concept clustering</strong> revealing emergent patterns</li>
              <li><strong>Incremental sync</strong> keeping the graph current</li>
            </ul>
          </div>

          {/* Reasoning Layer */}
          <div className="mb-10 p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <h3 className="text-2xl font-bold mb-4">4.3 Reasoning Layer (Tools + Workflows)</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-4">
              Where test-time computation happens:
            </p>
            <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
              <li><strong>Sequential Thinking</strong>: Multi-step reasoning with revision and branching</li>
              <li><strong>Workflow Orchestration</strong>: 30 distinct methodologies with quality gates and guardrails</li>
              <li><strong>Assumption Ledger</strong>: Traceable skepticism for agent reasoning—capture, validate, and track assumptions without auto-inference</li>
              <li><strong>Multi-agent Coordination</strong>: Wave-based execution with parallel agent dispatch</li>
              <li><strong>Intelligent Method Selection</strong>: Meta-cognitive system for optimal reasoning approach selection</li>
              <li><strong>RTM Validation</strong>: Requirements traceability for systematic development</li>
              <li><strong>Quality Control</strong>: Stop conditions, validation gates, and anti-slop controls</li>
            </ul>
            <p className="text-slate-700 dark:text-slate-300 mt-4 italic border-l-4 border-slate-300 dark:border-slate-600 pl-4">
              Context Window Economics: The PM (blue hat) uses sequential thinking to preserve expensive context while dispatching fresh agents for implementation. This allows complex projects without context exhaustion.
            </p>
            <p className="text-slate-600 dark:text-slate-400 mt-4 text-sm">
              ∴ The PM remembers so agents can forget
            </p>
          </div>
        </section>

        {/* Technical Specifications */}
        <section className="mb-12">
          <h2 className="text-3xl font-bold mb-6">Technical Specifications</h2>
          <div className="p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <ul className="space-y-3 text-slate-700 dark:text-slate-300">
              <li><strong>Language:</strong> C# with .NET 9.0</li>
              <li><strong>Vector Dimensions:</strong> 384 (all-MiniLM-L6-v2 via ONNX)</li>
              <li><strong>Search Algorithm:</strong> Reciprocal Rank Fusion (k=60)</li>
              <li><strong>Database:</strong> SQLite with vector extension</li>
              <li><strong>Graph Sync:</strong> Incremental with file watching</li>
              <li><strong>Memory Format:</strong> Markdown with YAML frontmatter</li>
              <li><strong>URI Scheme:</strong> <code className="text-blue-600 dark:text-blue-400">memory://</code> protocol</li>
              <li><strong>Tested Scale:</strong> &gt; 1.1 million relationships</li>
            </ul>
          </div>
        </section>

        {/* Cognitive Assets */}
        <section className="mb-12">
          <h2 className="text-3xl font-bold mb-6">Cognitive Assets</h2>
          <div className="p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <h3 className="text-xl font-bold mb-4">30 Distinct Methodologies</h3>
            <p className="text-slate-700 dark:text-slate-300 mb-4">
              Complete taxonomy from deductive reasoning to design thinking:
            </p>
            <ul className="space-y-2 text-slate-700 dark:text-slate-300 ml-4">
              <li><strong>Reasoning:</strong> deductive, inductive, abductive, critical, strategic, higher-order thinking</li>
              <li><strong>Creative:</strong> design thinking, divergent thinking, lateral thinking, oblique strategies, SCAMPER</li>
              <li><strong>Development:</strong> agentic-dev with anti-slop controls, agile, SDLC, code review workflows</li>
              <li><strong>Collaborative:</strong> world café, parallel thinking, six thinking hats</li>
              <li><strong>Meta-orchestration:</strong> workflow-dispatch for intelligent methodology selection</li>
            </ul>

            <div className="mt-6 pt-6 border-t border-slate-200 dark:border-slate-600">
              <h3 className="text-xl font-bold mb-4">Roles & Perspectives</h3>
              <ul className="space-y-1 text-slate-700 dark:text-slate-300 ml-4">
                <li><strong>7 Roles:</strong> architect, engineer, PM, data-scientist, product-manager, writer, designer</li>
                <li><strong>7 Thinking Hats:</strong> DeBono's Six Thinking Hats + Gray Hat</li>
              </ul>
            </div>
          </div>
        </section>

        {/* Key Capabilities */}
        <section className="mb-12">
          <h2 className="text-3xl font-bold mb-6">Key Capabilities</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
              <h3 className="font-bold mb-2 text-slate-900 dark:text-slate-50">Test-time Adaptive Reasoning</h3>
              <p className="text-sm text-slate-700 dark:text-slate-300">Sequential thinking with revision, branching, and multi-agent collaboration</p>
            </div>
            <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
              <h3 className="font-bold mb-2 text-slate-900 dark:text-slate-50">Intelligent Workflow Selection</h3>
              <p className="text-sm text-slate-700 dark:text-slate-300">Meta-cognitive system that analyzes problems and selects optimal reasoning approaches</p>
            </div>
            <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
              <h3 className="font-bold mb-2 text-slate-900 dark:text-slate-50">Hybrid RRF Search</h3>
              <p className="text-sm text-slate-700 dark:text-slate-300">Semantic + full-text fusion for optimal retrieval, not just embedding similarity</p>
            </div>
            <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
              <h3 className="font-bold mb-2 text-slate-900 dark:text-slate-50">Lazy Graph Construction</h3>
              <p className="text-sm text-slate-700 dark:text-slate-300">No schema, no ontology—structure emerges from WikiLink usage</p>
            </div>
            <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
              <h3 className="font-bold mb-2 text-slate-900 dark:text-slate-50">Quality-Gated Orchestration</h3>
              <p className="text-sm text-slate-700 dark:text-slate-300">Multi-agent coordination with validation waves, guardrails, and RTM compliance</p>
            </div>
            <div className="p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
              <h3 className="font-bold mb-2 text-slate-900 dark:text-slate-50">Complete Transparency</h3>
              <p className="text-sm text-slate-700 dark:text-slate-300">Every thought, revision, and decision visible in markdown files</p>
            </div>
          </div>
        </section>

        {/* How It Works Together */}
        <section className="mb-12">
          <h2 className="text-3xl font-bold mb-6">How They Work Together</h2>
          <div className="p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-blue-50 dark:bg-slate-900/50">
            <div className="space-y-4 text-slate-700 dark:text-slate-300">
              <p>
                The three layers integrate into a cohesive system for persistent, compound reasoning:
              </p>
              <ol className="list-decimal list-inside space-y-3 ml-2">
                <li>
                  <strong>Memory Layer</strong> captures every decision as markdown with WikiLinks
                </li>
                <li>
                  <strong>Graph Layer</strong> automatically builds relationships from those WikiLinks, creating a knowledge network
                </li>
                <li>
                  <strong>Reasoning Layer</strong> uses both memory and graph to enable multi-step thinking, intelligent workflow selection, and quality-gated orchestration
                </li>
              </ol>
              <p className="pt-4 italic">
                Together, they create a system where knowledge compounds over time, where reasoning can revise and branch, and where complex problems can be systematically solved.
              </p>
            </div>
          </div>
        </section>

        {/* MCP Compliance */}
        <section className="mb-12">
          <h2 className="text-3xl font-bold mb-6">MCP Integration</h2>
          <div className="p-6 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900">
            <p className="text-slate-700 dark:text-slate-300 mb-4">
              Maenifold is fully compliant with the Model Context Protocol, exposing all cognitive tools through the MCP interface:
            </p>
            <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300 ml-4">
              <li>Full tool annotation support for AI agents</li>
              <li>Seamless integration with Claude, Cursor, Continue, and other MCP clients</li>
              <li>Composable tools that can be orchestrated into complex workflows</li>
              <li>Real-time tool discovery and capability introspection</li>
            </ul>
          </div>
        </section>
      </div>
    </main>
  );
}
