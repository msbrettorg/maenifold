import React from 'react';

export const metadata = {
  title: 'Cognitive Assets',
  description: 'Pre-configured cognitive frameworks for AI agents',
};

export default function CognitiveAssetsPage() {
  return (
    <main className="min-h-screen bg-white dark:bg-slate-950 text-slate-900 dark:text-slate-50">
      <div className="max-w-6xl mx-auto px-4 py-16">
        {/* Header */}
        <h1 className="text-4xl font-bold mb-2">Cognitive Assets</h1>

        {/* Introduction */}
        <p className="text-lg text-slate-700 dark:text-slate-300 mb-12">
          maenifold ships with 54 pre-configured cognitive frameworks that agents can adopt dynamically:
        </p>

        {/* Grid Container */}
        <div className="space-y-12">
          {/* Workflows Section */}
          <section className="bg-slate-50 dark:bg-slate-900 rounded-lg p-8">
            <h2 className="text-2xl font-bold mb-1">
              <span className="mr-2">🔄</span>
              <span>28 Workflows - Structured Methodologies</span>
            </h2>
            <p className="text-slate-600 dark:text-slate-400 mb-4">
              From reasoning patterns to development processes:
            </p>
            <ul className="space-y-2 text-slate-700 dark:text-slate-300">
              <li>
                <strong>Reasoning:</strong> deductive, inductive, abductive, critical, strategic, higher-order thinking
              </li>
              <li>
                <strong>Creative:</strong> design thinking, divergent thinking, lateral thinking, oblique strategies, SCAMPER
              </li>
              <li>
                <strong>Development:</strong> agentic-dev with anti-slop controls, agile, SDLC, code review workflows
              </li>
              <li>
                <strong>Collaborative:</strong> world café, parallel thinking, six thinking hats
              </li>
              <li>
                <strong>Meta-orchestration:</strong> workflow-dispatch for intelligent methodology selection
              </li>
            </ul>
          </section>

          {/* Roles Section */}
          <section className="bg-slate-50 dark:bg-slate-900 rounded-lg p-8">
            <h2 className="text-2xl font-bold mb-1">
              <span className="mr-2">🎭</span>
              <span>7 Roles - Professional Perspectives</span>
            </h2>
            <p className="text-slate-600 dark:text-slate-400 mb-4">
              Each with personality, principles, and transition triggers:
            </p>
            <ul className="space-y-2 text-slate-700 dark:text-slate-300">
              <li>
                <strong>Product Manager:</strong> "Simple, Lovable, Complete" framework
              </li>
              <li>
                <strong>Architect:</strong> System design and patterns
              </li>
              <li>
                <strong>Engineer:</strong> Implementation and quality
              </li>
              <li>
                <strong>Researcher:</strong> Investigation and discovery
              </li>
              <li>
                <strong>Writer:</strong> Clear communication
              </li>
              <li>
                <strong>Red Team:</strong> Security testing and adversarial thinking
              </li>
              <li>
                <strong>Blue Team:</strong> Defense and protection strategies
              </li>
            </ul>
          </section>

          {/* Colors Section */}
          <section className="bg-slate-50 dark:bg-slate-900 rounded-lg p-8">
            <h2 className="text-2xl font-bold mb-1">
              <span className="mr-2">🎨</span>
              <span>7 Colors - De Bono's Six Hats + Gray</span>
            </h2>
            <p className="text-slate-600 dark:text-slate-400 mb-4">
              Thinking modes for different cognitive approaches:
            </p>
            <ul className="space-y-2 text-slate-700 dark:text-slate-300">
              <li>
                <strong>White:</strong> Facts and information
              </li>
              <li>
                <strong>Red:</strong> Emotions and intuition
              </li>
              <li>
                <strong>Black:</strong> Caution and critical judgment
              </li>
              <li>
                <strong>Yellow:</strong> Optimism and positive assessment
              </li>
              <li>
                <strong>Green:</strong> Creativity and alternatives
              </li>
              <li>
                <strong>Blue:</strong> Process control and orchestration
              </li>
              <li>
                <strong>Gray:</strong> Skeptical inquiry and assumption questioning
              </li>
            </ul>
          </section>

          {/* Perspectives Section */}
          <section className="bg-slate-50 dark:bg-slate-900 rounded-lg p-8">
            <h2 className="text-2xl font-bold mb-1">
              <span className="mr-2">🗣️</span>
              <span>12 Perspectives - Sapir-Whorf Linguistic Patterns</span>
            </h2>
            <p className="text-slate-600 dark:text-slate-400 mb-4">
              Language-influenced cognitive styles based on linguistic relativity:
            </p>
            <ul className="space-y-2 text-slate-700 dark:text-slate-300">
              <li>
                <strong>Arabic:</strong> Pattern analysis through root systems
              </li>
              <li>
                <strong>Japanese:</strong> Contextual and relational thinking
              </li>
              <li>
                <strong>German:</strong> Hierarchical precision
              </li>
              <li>
                <strong>And 9 more:</strong> Each language brings unique reasoning patterns
              </li>
            </ul>
          </section>

          {/* Integration Note */}
          <section className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-8">
            <p className="text-slate-700 dark:text-slate-300">
              These aren't just templates - they're cognitive substrates that combine with the knowledge graph to create emergent reasoning capabilities. An agent can start with strategic thinking, switch to red team perspective when finding vulnerabilities, then adopt the writer role to document findings - all while building on the persistent knowledge graph.
            </p>
          </section>
        </div>
      </div>
    </main>
  );
}
