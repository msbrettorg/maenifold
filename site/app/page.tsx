import Link from 'next/link';

export default function Home() {
  return (
    <div className="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">
      {/* Hero Section */}
      <section className="relative min-h-screen flex items-center justify-center px-6 py-20 bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
        <div className="max-w-4xl mx-auto text-center">
          <div className="mb-8">
            <h1 className="text-5xl md:text-7xl font-bold mb-6 bg-gradient-to-r from-blue-600 to-purple-600 dark:from-blue-400 dark:to-purple-400 bg-clip-text text-transparent">
              Your AI is ephemeral.
            </h1>
            <h2 className="text-4xl md:text-5xl font-bold mb-8">
              Your knowledge shouldn't be.
            </h2>
          </div>

          <p className="text-xl md:text-2xl text-slate-700 dark:text-slate-300 mb-12 leading-relaxed max-w-2xl mx-auto">
            <strong>maenifold</strong> enhances AI agents with persistent graphs of thought that compound over time. Every tool creates WikiLink connections that survive conversations. Every session builds on the last. Knowledge compounds instead of resets.
          </p>

          <p className="text-lg text-slate-600 dark:text-slate-400 mb-12 italic">
            It transforms ephemeral AI sessions into continuous collective intelligence.
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center mb-16">
            <Link
              href="/start"
              className="px-8 py-4 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 text-white font-bold rounded-lg transition text-lg"
            >
              Get Started
            </Link>
            <Link
              href="/docs/architecture"
              className="px-8 py-4 border-2 border-blue-600 dark:border-blue-400 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-slate-800 font-bold rounded-lg transition text-lg"
            >
              Learn Architecture
            </Link>
          </div>

          <div className="rounded-lg overflow-hidden shadow-xl border border-slate-200 dark:border-slate-700">
            <img
              src="/maenifold/assets/branding/graph.jpeg"
              alt="Knowledge graph visualization"
              className="w-full h-auto"
            />
          </div>
        </div>
      </section>

      {/* What maenifold Does */}
      <section className="py-20 px-6 bg-slate-50 dark:bg-slate-800">
        <div className="max-w-4xl mx-auto">
          <h2 className="text-4xl font-bold mb-12 text-center">What maenifold Does</h2>

          <div className="grid md:grid-cols-2 gap-8">
            <div className="bg-white dark:bg-slate-700 p-8 rounded-lg shadow">
              <h3 className="text-2xl font-bold mb-4 text-blue-600 dark:text-blue-400">Persistent Knowledge</h3>
              <p className="text-lg text-slate-700 dark:text-slate-300">
                Every tool creates <code className="bg-slate-100 dark:bg-slate-600 px-2 py-1 rounded text-purple-600 dark:text-purple-400">[[WikiLink]]</code> connections that survive conversations. Knowledge compounds instead of resets.
              </p>
            </div>

            <div className="bg-white dark:bg-slate-700 p-8 rounded-lg shadow">
              <h3 className="text-2xl font-bold mb-4 text-blue-600 dark:text-blue-400">Shared Graphs</h3>
              <p className="text-lg text-slate-700 dark:text-slate-300">
                Every session builds on the last. Multiple AI agents can work from the same knowledge graph, seeing discoveries made by previous agents.
              </p>
            </div>

            <div className="bg-white dark:bg-slate-700 p-8 rounded-lg shadow">
              <h3 className="text-2xl font-bold mb-4 text-blue-600 dark:text-blue-400">Emergent Intelligence</h3>
              <p className="text-lg text-slate-700 dark:text-slate-300">
                Patterns emerge from the intersection of multiple test sessions and agent discoveries. Intelligence compounds as the graph grows.
              </p>
            </div>

            <div className="bg-white dark:bg-slate-700 p-8 rounded-lg shadow">
              <h3 className="text-2xl font-bold mb-4 text-blue-600 dark:text-blue-400">Zero Orchestration</h3>
              <p className="text-lg text-slate-700 dark:text-slate-300">
                Agents coordinate through the shared knowledge graph with no orchestration code. Intelligence fills the space.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Demo Story */}
      <section className="py-20 px-6 bg-white dark:bg-slate-900">
        <div className="max-w-4xl mx-auto">
          <h2 className="text-4xl font-bold mb-12 text-center">The Proof: A Real Demo</h2>

          <div className="bg-blue-50 dark:bg-slate-800 border-2 border-blue-200 dark:border-blue-900 rounded-lg p-8 mb-8">
            <p className="text-xl text-slate-800 dark:text-slate-200 mb-6">
              Our demo shows this at scale: <strong>25 AI agents across 68 minutes</strong> discovered a critical production bug not through programmed coordination, but through emergent understanding.
            </p>

            <div className="grid md:grid-cols-3 gap-6 mb-8">
              <div className="text-center">
                <div className="text-5xl font-bold text-blue-600 dark:text-blue-400 mb-2">25</div>
                <p className="text-slate-700 dark:text-slate-300">AI Agents</p>
              </div>
              <div className="text-center">
                <div className="text-5xl font-bold text-blue-600 dark:text-blue-400 mb-2">68</div>
                <p className="text-slate-700 dark:text-slate-300">Minutes</p>
              </div>
              <div className="text-center">
                <div className="text-5xl font-bold text-blue-600 dark:text-blue-400 mb-2">85%</div>
                <p className="text-slate-700 dark:text-slate-300">Success Rate</p>
              </div>
            </div>

            <p className="text-slate-700 dark:text-slate-300 mb-6">
              <strong>No orchestration code was written, yet agents perfectly orchestrated themselves across 4 waves, building on each other's discoveries through a shared knowledge graph.</strong>
            </p>

            <p className="text-slate-700 dark:text-slate-300 mb-6">
              The critical move operation bug emerged from the intersection of multiple test patterns seen across different agent sessions - something no single agent could have found alone. This is maenifold's core: making every AI session additive rather than isolated.
            </p>
          </div>

          <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-8">
            <h3 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">How It Worked:</h3>
            <ul className="space-y-4">
              <li className="flex items-start">
                <span className="text-2xl mr-4 text-blue-600">✓</span>
                <span className="text-slate-700 dark:text-slate-300">Agents shared discoveries through <code className="bg-white dark:bg-slate-700 px-2 py-1 rounded text-purple-600 dark:text-purple-400">[[WikiLinks]]</code> in memory files</span>
              </li>
              <li className="flex items-start">
                <span className="text-2xl mr-4 text-blue-600">✓</span>
                <span className="text-slate-700 dark:text-slate-300">Each wave of agents built on previous findings via search and context traversal</span>
              </li>
              <li className="flex items-start">
                <span className="text-2xl mr-4 text-blue-600">✓</span>
                <span className="text-slate-700 dark:text-slate-300">The critical bug emerged from patterns across multiple test sessions</span>
              </li>
              <li className="flex items-start">
                <span className="text-2xl mr-4 text-blue-600">✓</span>
                <span className="text-slate-700 dark:text-slate-300">171,506 new concept relationships were created, connecting discoveries</span>
              </li>
            </ul>

            <p className="text-slate-700 dark:text-slate-300 mt-6 text-center italic">
              <strong>Real production bug found. Zero orchestration code written.</strong>
            </p>
          </div>
        </div>
      </section>

      {/* Cognitive Assets */}
      <section className="py-20 px-6 bg-slate-50 dark:bg-slate-800">
        <div className="max-w-6xl mx-auto">
          <h2 className="text-4xl font-bold mb-4 text-center">Cognitive Assets</h2>
          <p className="text-xl text-center text-slate-700 dark:text-slate-300 mb-12 max-w-2xl mx-auto">
            maenifold ships with <strong>54 pre-configured cognitive frameworks</strong> that agents can adopt dynamically
          </p>

          <div className="grid md:grid-cols-2 gap-8 mb-8">
            {/* Workflows */}
            <div className="bg-white dark:bg-slate-700 rounded-lg p-8 shadow">
              <h3 className="text-2xl font-bold mb-4 flex items-center">
                <span className="text-3xl mr-3">🔄</span>
                <span>28 Workflows</span>
              </h3>
              <p className="text-slate-700 dark:text-slate-300 mb-4">Structured Methodologies</p>
              <ul className="space-y-2 text-slate-700 dark:text-slate-300">
                <li><strong>Reasoning:</strong> deductive, inductive, abductive, critical, strategic, higher-order thinking</li>
                <li><strong>Creative:</strong> design thinking, divergent thinking, lateral thinking, oblique strategies, SCAMPER</li>
                <li><strong>Development:</strong> agentic-dev, agile, SDLC, code review workflows</li>
                <li><strong>Collaborative:</strong> world café, parallel thinking, six thinking hats</li>
                <li><strong>Meta:</strong> workflow-dispatch for intelligent methodology selection</li>
              </ul>
            </div>

            {/* Roles */}
            <div className="bg-white dark:bg-slate-700 rounded-lg p-8 shadow">
              <h3 className="text-2xl font-bold mb-4 flex items-center">
                <span className="text-3xl mr-3">🎭</span>
                <span>7 Roles</span>
              </h3>
              <p className="text-slate-700 dark:text-slate-300 mb-4">Professional Perspectives</p>
              <ul className="space-y-2 text-slate-700 dark:text-slate-300">
                <li><strong>Product Manager:</strong> "Simple, Lovable, Complete" framework</li>
                <li><strong>Architect:</strong> System design and patterns</li>
                <li><strong>Engineer:</strong> Implementation and quality</li>
                <li><strong>Researcher:</strong> Investigation and discovery</li>
                <li><strong>Writer:</strong> Clear communication</li>
                <li><strong>Red Team:</strong> Security testing and adversarial thinking</li>
                <li><strong>Blue Team:</strong> Defense and protection strategies</li>
              </ul>
            </div>

            {/* Colors */}
            <div className="bg-white dark:bg-slate-700 rounded-lg p-8 shadow">
              <h3 className="text-2xl font-bold mb-4 flex items-center">
                <span className="text-3xl mr-3">🎨</span>
                <span>7 Colors</span>
              </h3>
              <p className="text-slate-700 dark:text-slate-300 mb-4">De Bono's Six Hats + Gray</p>
              <ul className="space-y-2 text-slate-700 dark:text-slate-300">
                <li><strong>White:</strong> Facts and information</li>
                <li><strong>Red:</strong> Emotions and intuition</li>
                <li><strong>Black:</strong> Caution and critical judgment</li>
                <li><strong>Yellow:</strong> Optimism and positive assessment</li>
                <li><strong>Green:</strong> Creativity and alternatives</li>
                <li><strong>Blue:</strong> Process control and orchestration</li>
                <li><strong>Gray:</strong> Skeptical inquiry and assumption questioning</li>
              </ul>
            </div>

            {/* Perspectives */}
            <div className="bg-white dark:bg-slate-700 rounded-lg p-8 shadow">
              <h3 className="text-2xl font-bold mb-4 flex items-center">
                <span className="text-3xl mr-3">🗣️</span>
                <span>12 Perspectives</span>
              </h3>
              <p className="text-slate-700 dark:text-slate-300 mb-4">Sapir-Whorf Linguistic Patterns</p>
              <p className="text-slate-700 dark:text-slate-300">
                Language-influenced cognitive styles based on linguistic relativity. Each language brings unique reasoning patterns including Arabic (pattern analysis), Japanese (contextual thinking), German (hierarchical precision), and 9 more.
              </p>
            </div>
          </div>

          <div className="bg-blue-50 dark:bg-slate-700 border-l-4 border-blue-600 rounded p-8">
            <p className="text-slate-800 dark:text-slate-200">
              <strong>These aren't just templates</strong> — they're cognitive substrates that combine with the knowledge graph to create emergent reasoning capabilities. An agent can start with strategic thinking, switch to red team perspective when finding vulnerabilities, then adopt the writer role to document findings — all while building on the persistent knowledge graph.
            </p>
          </div>
        </div>
      </section>

      {/* The Cognitive Stack */}
      <section className="py-20 px-6 bg-white dark:bg-slate-900">
        <div className="max-w-4xl mx-auto">
          <h2 className="text-4xl font-bold mb-12 text-center">The Cognitive Stack</h2>

          <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-8 mb-12">
            <h3 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Three Layers Working Together</h3>

            <div className="space-y-8">
              <div>
                <h4 className="text-xl font-bold text-blue-600 dark:text-blue-400 mb-3">Reasoning Layer (Tools + Workflows)</h4>
                <p className="text-slate-700 dark:text-slate-300 mb-3">
                  Where information is processed with test-time computation:
                </p>
                <ul className="space-y-2 text-slate-700 dark:text-slate-300 ml-4">
                  <li>• Sequential thinking with revision, branching, persistence and automatic graph construction</li>
                  <li>• Rich role and color definitions provide tunable agent perspectives</li>
                  <li>• Intelligent workflow selection analyzes problems and selects optimal reasoning approaches</li>
                  <li>• 30+ distinct methodologies from deductive reasoning to design thinking</li>
                  <li>• Assumption Ledger for traceable skepticism in agent reasoning</li>
                  <li>• Multi-agent coordination with wave-based execution and parallel dispatch</li>
                </ul>
              </div>

              <div>
                <h4 className="text-xl font-bold text-blue-600 dark:text-blue-400 mb-3">Memory Layer (memory://)</h4>
                <p className="text-slate-700 dark:text-slate-300 mb-3">
                  Where data is stored:
                </p>
                <ul className="space-y-2 text-slate-700 dark:text-slate-300 ml-4">
                  <li>• <strong>Local:</strong> Every piece of knowledge lives as a markdown file on disk with a unique URI</li>
                  <li>• <strong>Transparent:</strong> Every thought, revision, and decision visible in markdown files</li>
                  <li>• <strong>Human-Friendly:</strong> All files are human-readable, Obsidian-compatible, persist across sessions</li>
                </ul>
              </div>

              <div>
                <h4 className="text-xl font-bold text-blue-600 dark:text-blue-400 mb-3">Graph Layer (SQLite + vectors)</h4>
                <p className="text-slate-700 dark:text-slate-300 mb-3">
                  Where knowledge emerges:
                </p>
                <ul className="space-y-2 text-slate-700 dark:text-slate-300 ml-4">
                  <li>• Automatic graph construction from <code className="bg-white dark:bg-slate-700 px-2 py-1 rounded text-purple-600 dark:text-purple-400">[[WikiLinks]]</code> with 384-dimensional embeddings</li>
                  <li>• Edge weights strengthen with repeated mentions</li>
                  <li>• Concept clustering reveals emergent patterns</li>
                  <li>• Incremental sync keeps the graph current</li>
                  <li>• Hybrid RRF search: semantic + full-text fusion</li>
                  <li>• No schema, no ontology — structure emerges from WikiLink usage</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Technical Specifications */}
      <section className="py-20 px-6 bg-slate-50 dark:bg-slate-800">
        <div className="max-w-4xl mx-auto">
          <h2 className="text-4xl font-bold mb-12 text-center">Technical Specifications</h2>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Language:</strong> C# with .NET 9.0</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Vector Dimensions:</strong> 384 (all-MiniLM-L6-v2 via ONNX)</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Search Algorithm:</strong> Reciprocal Rank Fusion (k=60)</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Database:</strong> SQLite with vector extension</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Graph Sync:</strong> Incremental with file watching</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Memory Format:</strong> Markdown with YAML frontmatter</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>URI Scheme:</strong> <code>memory://</code> protocol</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg">
              <p className="text-slate-700 dark:text-slate-300"><strong>Tested Scale:</strong> &gt; 1.1 million relationships</p>
            </div>
            <div className="bg-white dark:bg-slate-700 p-6 rounded-lg col-span-2">
              <p className="text-slate-700 dark:text-slate-300"><strong>MCP Compliance:</strong> Full tool annotation support</p>
            </div>
          </div>
        </div>
      </section>

      {/* Final CTA */}
      <section className="py-20 px-6 bg-gradient-to-r from-blue-600 to-purple-600 dark:from-blue-900 dark:to-purple-900">
        <div className="max-w-4xl mx-auto text-center text-white">
          <h2 className="text-4xl font-bold mb-6">Ready to Transform Your AI?</h2>
          <p className="text-xl mb-8 text-blue-100">
            Start building with persistent knowledge graphs today
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <Link
              href="/start"
              className="px-8 py-4 bg-white text-blue-600 hover:bg-blue-50 font-bold rounded-lg transition text-lg"
            >
              Get Started Now
            </Link>
            <Link
              href="/docs/philosophy"
              className="px-8 py-4 border-2 border-white text-white hover:bg-white hover:bg-opacity-10 font-bold rounded-lg transition text-lg"
            >
              Learn Our Philosophy
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
