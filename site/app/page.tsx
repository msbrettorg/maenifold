import Image from "next/image";

export default function Home() {
  return (
    <div className="min-h-screen bg-white dark:bg-black text-black dark:text-white">
      <main className="max-w-4xl mx-auto px-4 py-16 sm:py-24">
        {/* Hero Section */}
        <div className="flex flex-col items-center gap-8 mb-16">
          {/* Logo */}
          <div className="flex justify-center">
            <img
              src="/assets/branding/maenifold-logo.svg"
              alt="maenifold"
              className="h-32 w-auto"
            />
          </div>

          {/* Tagline */}
          <div className="text-center">
            <p className="text-xl sm:text-2xl font-light text-gray-700 dark:text-gray-300">
              Your AI is ephemeral. Your knowledge shouldn't be.
            </p>
          </div>
        </div>

        {/* What maenifold does Section */}
        <section className="space-y-8">
          <h2 className="text-3xl font-bold">What maenifold does</h2>

          <div className="space-y-6 text-lg leading-relaxed text-gray-800 dark:text-gray-200">
            <p>
              <strong>maenifold</strong> enhances AI agents with persistent graphs of thought that compound over time. Every tool creates <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-sm">[[WikiLink]]</code> connections that survive conversations. Every session builds on the last. Knowledge compounds instead of resets. <strong>It transforms ephemeral AI sessions into continuous collective intelligence.</strong>
            </p>

            {/* Graph Image */}
            <div className="flex justify-center py-8">
              <img
                src="/assets/branding/graph.jpeg"
                alt="graph"
                className="max-w-full h-auto rounded-lg shadow-lg"
              />
            </div>

            <p>
              <a href="/assets/demo-artifacts/README.md" className="text-blue-600 dark:text-blue-400 hover:underline">
                Our demo
              </a> shows this at scale: 25 AI agents across 68 minutes discovered a critical production bug not through programmed coordination, but through emergent understanding. No orchestration code was written, yet agents perfectly orchestrated themselves across 4 waves, building on each other's discoveries through a shared knowledge graph.
            </p>

            <p>
              The critical move operation bug emerged from the intersection of multiple test patterns seen across different agent sessions - something no single agent could have found alone. This is maenifold's core: making every AI session additive rather than isolated.
            </p>

            <h3 className="text-xl font-semibold mt-6">How it worked:</h3>
            <ul className="list-disc list-inside space-y-2 text-gray-800 dark:text-gray-200">
              <li>Agents shared discoveries through <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-sm">[[WikiLinks]]</code> in memory files</li>
              <li>Each wave of agents built on previous findings via search and context traversal</li>
              <li>The critical bug emerged from patterns across multiple test sessions</li>
              <li>171,506 new concept relationships were created, connecting discoveries</li>
              <li>
                <a href="/assets/demo-artifacts/part1-pm-lite/orchestration-session.md" className="text-blue-600 dark:text-blue-400 hover:underline">
                  Full orchestration logs
                </a> and <a href="/assets/demo-artifacts/part1-pm-lite/E2E_TEST_REPORT.md" className="text-blue-600 dark:text-blue-400 hover:underline">
                  test results
                </a> available
              </li>
            </ul>

            <p className="text-base font-semibold text-gray-900 dark:text-gray-100 pt-4">
              85% test success rate. Real production bug found. Zero orchestration code written.
            </p>

            <p>
              That's what maenifold does: It provides the substrate (WikiLinks, memory, graph) and steps back. Intelligence fills the space.
            </p>
          </div>
        </section>
      </main>
    </div>
  );
}
