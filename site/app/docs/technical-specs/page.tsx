'use client';

export default function TechnicalSpecsPage() {
  return (
    <div className="min-h-screen bg-white dark:bg-gray-950 transition-colors">
      <div className="max-w-4xl mx-auto px-4 py-12">
        {/* Breadcrumbs */}
        <nav className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 mb-8">
          <a href="/" className="hover:text-gray-900 dark:hover:text-gray-200">Home</a>
          <span>/</span>
          <a href="/docs/architecture" className="hover:text-gray-900 dark:hover:text-gray-200">Docs</a>
          <span>/</span>
          <span className="text-gray-900 dark:text-gray-200">Technical Specs</span>
        </nav>

        {/* Page Header */}
        <header className="mb-12">
          <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-4">
            Technical Specifications
          </h1>
          <p className="text-lg text-gray-600 dark:text-gray-400">
            The engineering foundation that powers maenifold's persistent knowledge graph.
          </p>
        </header>

        {/* Core Technology Stack */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            Core Technology Stack
          </h2>

          <div className="bg-gray-50 dark:bg-gray-900 rounded-lg p-6 mb-6 border border-gray-200 dark:border-gray-800">
            <dl className="space-y-4">
              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Language</dt>
                <dd className="text-gray-600 dark:text-gray-400">C# with .NET 9.0</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Database</dt>
                <dd className="text-gray-600 dark:text-gray-400">SQLite with vector extension</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Vector Embeddings</dt>
                <dd className="text-gray-600 dark:text-gray-400">384 dimensions (all-MiniLM-L6-v2 via ONNX)</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Search Algorithm</dt>
                <dd className="text-gray-600 dark:text-gray-400">Reciprocal Rank Fusion (RRF, k=60) with Hybrid search (semantic + full-text)</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Graph Sync</dt>
                <dd className="text-gray-600 dark:text-gray-400">Incremental with file watching</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Memory Format</dt>
                <dd className="text-gray-600 dark:text-gray-400">Markdown with YAML frontmatter</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">URI Scheme</dt>
                <dd className="text-gray-600 dark:text-gray-400">memory:// protocol</dd>
              </div>

              <div>
                <dt className="font-semibold text-gray-900 dark:text-white">Protocol</dt>
                <dd className="text-gray-600 dark:text-gray-400">MCP (Model Context Protocol) with full tool annotation support</dd>
              </div>
            </dl>
          </div>
        </section>

        {/* Scale & Performance */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            Scale Testing Results
          </h2>

          <div className="bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-gray-900 dark:to-gray-800 rounded-lg p-6 border border-blue-200 dark:border-gray-700 mb-6">
            <div className="text-center">
              <div className="text-5xl font-bold text-blue-600 dark:text-blue-400 mb-2">
                1.1M+
              </div>
              <p className="text-gray-700 dark:text-gray-300">
                Relationships tested and verified
              </p>
            </div>
          </div>

          <div className="space-y-4">
            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Graph Performance</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm">
                maenifold has been stress-tested with over 1.1 million concept relationships. The graph layer efficiently handles:
              </p>
              <ul className="list-disc list-inside text-gray-600 dark:text-gray-400 text-sm mt-3 space-y-1">
                <li>Incremental concept updates with automatic relationship detection</li>
                <li>Semantic similarity search across large knowledge bases</li>
                <li>Hybrid RRF search combining semantic and full-text results</li>
                <li>Concurrent agent access to the same memory location</li>
              </ul>
            </div>

            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Demonstrated Capability</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm">
                In production demonstrations, maenifold supported 25 AI agents across 68 minutes of coordinated discovery, creating 171,506 new concept relationships without any orchestration code—demonstrating emergent multi-agent coordination at scale.
              </p>
            </div>
          </div>
        </section>

        {/* Graph Architecture */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            Graph Layer Architecture
          </h2>

          <div className="space-y-4">
            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-3">Automatic Graph Construction</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm mb-3">
                The graph emerges from WikiLink usage without pre-defined schema or ontology:
              </p>
              <ul className="space-y-2">
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>WikiLink Detection:</strong> Every [[Concept]] reference in markdown files automatically creates a graph node
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Semantic Embeddings:</strong> Each concept is vectorized in 384-dimensional space for similarity search
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Edge Weights:</strong> Relationship strength increases with repeated mentions across memory files
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Concept Clustering:</strong> Emergent patterns reveal knowledge structure and relationships
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Incremental Sync:</strong> File watching ensures the graph stays synchronized with memory updates
                </li>
              </ul>
            </div>

            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-3">Search & Discovery</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm mb-3">
                Multiple search modes optimize for different retrieval patterns:
              </p>
              <ul className="space-y-2">
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Hybrid RRF Search:</strong> Fuses semantic similarity and full-text matching using Reciprocal Rank Fusion (k=60)
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Semantic Search:</strong> 384-dimensional embeddings find conceptually related content
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Full-Text Search:</strong> Traditional regex-based pattern matching for exact content queries
                </li>
                <li className="text-gray-600 dark:text-gray-400 text-sm">
                  <strong>Context Building:</strong> Graph traversal reveals connected knowledge at any depth
                </li>
              </ul>
            </div>
          </div>
        </section>

        {/* MCP Compliance */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            MCP Compliance & Integration
          </h2>

          <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800 mb-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-3">Protocol Support</h3>
            <p className="text-gray-600 dark:text-gray-400 text-sm mb-4">
              maenifold is fully compliant with the Model Context Protocol (MCP), providing a standardized interface for AI tools and applications:
            </p>
            <ul className="space-y-2">
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Tool Annotation:</strong> Full support for MCP tool specifications with complete metadata
              </li>
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Resource Handling:</strong> Proper resource management for long-lived agent interactions
              </li>
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Workflow Integration:</strong> Seamless orchestration with MCP-compatible AI frameworks
              </li>
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Feature Parity:</strong> MCP and CLI interfaces have identical capabilities—start via MCP, continue via CLI
              </li>
            </ul>
          </div>

          <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-3">Supported Platforms</h3>
            <p className="text-gray-600 dark:text-gray-400 text-sm mb-4">
              maenifold integrates with major AI tools and platforms:
            </p>
            <ul className="space-y-2">
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Claude Code, Continue, Cline:</strong> Full MCP integration via stdio protocol
              </li>
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Codex:</strong> TOML configuration with configurable timeouts
              </li>
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>CLI Interface:</strong> Direct command-line access for scripts and pipelines
              </li>
              <li className="text-gray-600 dark:text-gray-400 text-sm">
                <strong>Concurrent Agent Access:</strong> Multiple agents can read/write the same memory location safely
              </li>
            </ul>
          </div>
        </section>

        {/* Architecture Principles */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            Architectural Principles
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Local First</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm">
                All knowledge lives as markdown files on disk with persistent URIs. No cloud dependency, no data leaving your system.
              </p>
            </div>

            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Transparent</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm">
                Every thought, revision, and decision is visible in human-readable markdown. No hidden internal state or opaque operations.
              </p>
            </div>

            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Composable</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm">
                Multiple agents can safely access the same knowledge base concurrently. No conflicts, no coordination code needed.
              </p>
            </div>

            <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Emergent</h3>
              <p className="text-gray-600 dark:text-gray-400 text-sm">
                Graph structure emerges from WikiLink usage without pre-defined schema. Organization arises naturally from how agents think.
              </p>
            </div>
          </div>
        </section>

        {/* Performance Characteristics */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            Performance Characteristics
          </h2>

          <div className="bg-white dark:bg-gray-900 rounded-lg p-6 border border-gray-200 dark:border-gray-800">
            <div className="space-y-4">
              <div className="pb-4 border-b border-gray-200 dark:border-gray-700">
                <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Memory Operations</h3>
                <p className="text-gray-600 dark:text-gray-400 text-sm">
                  Write, read, and search operations are optimized for both small personal knowledge bases and large multi-agent systems. Incremental file watching prevents expensive full-graph rebuilds.
                </p>
              </div>

              <div className="pb-4 border-b border-gray-200 dark:border-gray-700">
                <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Graph Operations</h3>
                <p className="text-gray-600 dark:text-gray-400 text-sm">
                  The graph layer efficiently handles incremental updates. New WikiLinks are detected and indexed automatically without blocking concurrent access.
                </p>
              </div>

              <div>
                <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Scalability</h3>
                <p className="text-gray-600 dark:text-gray-400 text-sm">
                  Demonstrated capability at scale: 1.1M+ relationships, 25 concurrent agents, 171,506 new concept relationships created in a single demonstration run.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* Additional Resources */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
            Additional Resources
          </h2>

          <div className="bg-blue-50 dark:bg-gray-900 rounded-lg p-6 border border-blue-200 dark:border-gray-700">
            <p className="text-gray-700 dark:text-gray-300 text-sm mb-4">
              For more detailed information about maenifold's architecture and design philosophy:
            </p>
            <div className="space-y-2">
              <p className="text-sm">
                <a href="/docs/architecture" className="text-blue-600 dark:text-blue-400 hover:underline">
                  → Architecture & Cognitive Stack
                </a>
              </p>
              <p className="text-sm">
                <a href="/docs/philosophy" className="text-blue-600 dark:text-blue-400 hover:underline">
                  → Philosophy & Design Principles
                </a>
              </p>
              <p className="text-sm">
                <a href="/start" className="text-blue-600 dark:text-blue-400 hover:underline">
                  → Quick Start Guide
                </a>
              </p>
            </div>
          </div>
        </section>

        {/* Footer CTA */}
        <footer className="border-t border-gray-200 dark:border-gray-800 pt-8 mt-12">
          <p className="text-gray-600 dark:text-gray-400 text-sm text-center">
            maenifold runs on .NET 9.0 · SQLite · ONNX · MCP protocol
          </p>
        </footer>
      </div>
    </div>
  );
}
