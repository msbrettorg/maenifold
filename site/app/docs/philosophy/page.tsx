import Link from 'next/link';

export const metadata = {
  title: 'Philosophy - Maenifold',
  description: 'Ma Protocol principles and design philosophy',
};

export default function PhilosophyPage() {
  return (
    <div className="min-h-screen bg-white dark:bg-slate-900 text-slate-900 dark:text-white">
      {/* Breadcrumbs */}
      <div className="max-w-4xl mx-auto px-4 py-4 text-sm">
        <div className="flex items-center gap-2 text-slate-600 dark:text-slate-400">
          <Link href="/" className="hover:text-slate-900 dark:hover:text-white">Home</Link>
          <span>/</span>
          <Link href="/docs/architecture" className="hover:text-slate-900 dark:hover:text-white">Docs</Link>
          <span>/</span>
          <span className="text-slate-900 dark:text-white">Philosophy</span>
        </div>
      </div>

      {/* Main Content */}
      <article className="max-w-4xl mx-auto px-4 py-12">
        <h1 className="text-4xl font-bold mb-8">Philosophy</h1>

        {/* Ma Protocol Section */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold mb-6">The Ma Protocol</h2>

          <blockquote className="border-l-4 border-blue-500 pl-6 py-4 my-6 italic text-lg bg-slate-50 dark:bg-slate-800 rounded">
            "The space between the notes is where the music lives."
          </blockquote>

          <p className="text-lg mb-6">
            We believe in the radical power of emptiness.
          </p>

          <p className="mb-4">
            In a world drowning in features, we choose space.<br />
            In a culture obsessed with additions, we celebrate subtractions.<br />
            In systems that decide for users, we create silence for AI to think.
          </p>

          <p className="mb-6">
            <strong>Ma (間)</strong> is not just a design principle—it's a philosophy of software that recognizes the space between things as the thing itself.
          </p>

          {/* Convictions */}
          <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-6 my-6">
            <h3 className="text-xl font-bold mb-4">Our Convictions</h3>
            <ul className="space-y-3">
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Every feature we don't add creates room for intelligence to emerge</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Every decision we don't make preserves agency for those who use our tools</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Every line of code we don't write is a gift to maintainers yet unknown</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Every configuration we don't require is freedom preserved</span>
              </li>
            </ul>
          </div>

          {/* Practice */}
          <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-6 my-6">
            <h3 className="text-xl font-bold mb-4">Our Practice</h3>
            <ul className="space-y-3">
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>We ship defaults that work, knowing they can be overridden</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>We resist the siren call of clever abstraction</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>We document what we don't do as carefully as what we do</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>We measure success not by adoption metrics but by the intelligence we enable</span>
              </li>
            </ul>
          </div>

          {/* Promise */}
          <div className="bg-blue-50 dark:bg-slate-800 border border-blue-200 dark:border-blue-900 rounded-lg p-6 my-6">
            <h3 className="text-xl font-bold mb-4">Our Promise</h3>
            <p className="mb-4 font-semibold">To those who seek bloated frameworks: <span className="text-red-600">We are not for you.</span></p>
            <p className="mb-4 font-semibold">To those who want magic abstractions: <span className="text-red-600">Look elsewhere.</span></p>
            <p className="mb-6 font-semibold">To those who need hand-holding: <span className="text-red-600">This is not your path.</span></p>

            <p className="mb-4">But...</p>
            <p className="mb-3">To those who understand that true power lies in restraint,</p>
            <p className="mb-3">To those who see beauty in empty directories,</p>
            <p className="mb-3">To those who know that the best code is no code,</p>
            <p className="mb-6">To those who want tools that amplify rather than replace intelligence:</p>

            <p className="text-lg font-bold text-blue-600 dark:text-blue-400">Welcome to Ma.</p>
          </div>

          <p className="text-center text-slate-600 dark:text-slate-400 italic py-4">
            This is software that doesn't try to be smart because it's too busy creating space for brilliance.
          </p>
        </section>

        {/* What We Don't Do Section */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold mb-6">What We Don't Do</h2>

          <p className="mb-6">
            This section celebrates the features we've resisted adding. Each absence creates space (Ma) for AI to work more effectively.
          </p>

          {/* Core Principles */}
          <div className="space-y-6 mb-8">
            <div className="bg-red-50 dark:bg-slate-800 border border-red-200 dark:border-red-900 rounded-lg p-6">
              <h3 className="text-xl font-bold mb-3 text-red-700 dark:text-red-400">NO FAKE AI</h3>
              <p className="text-sm text-slate-600 dark:text-slate-300 mb-3">
                We don't add retry logic, fallback strategies, "smart" error recovery, adaptive behavior based on failures, or decisions about what you "probably meant."
              </p>
              <p className="text-sm italic">
                <strong>Why:</strong> Every decision we make removes a decision the LLM could make. The LLM has context we don't.
              </p>
            </div>

            <div className="bg-red-50 dark:bg-slate-800 border border-red-200 dark:border-red-900 rounded-lg p-6">
              <h3 className="text-xl font-bold mb-3 text-red-700 dark:text-red-400">NO UNNECESSARY ABSTRACTIONS</h3>
              <p className="text-sm text-slate-600 dark:text-slate-300 mb-3">
                We don't create abstraction layers, factory patterns, dependency injection frameworks, abstract base classes for single implementations, or hide simple operations behind complex APIs.
              </p>
              <p className="text-sm italic">
                <strong>Why:</strong> Abstractions create cognitive load. Simplicity creates understanding.
              </p>
            </div>

            <div className="bg-red-50 dark:bg-slate-800 border border-red-200 dark:border-red-900 rounded-lg p-6">
              <h3 className="text-xl font-bold mb-3 text-red-700 dark:text-red-400">NO PREMATURE OPTIMIZATION</h3>
              <p className="text-sm text-slate-600 dark:text-slate-300 mb-3">
                We don't add caching layers "just in case", implement connection pooling by default, create background workers for "faster processing", batch operations that work fine individually, or optimize code that isn't proven slow.
              </p>
              <p className="text-sm italic">
                <strong>Why:</strong> Premature optimization removes flexibility. The LLM might need that "inefficient" behavior.
              </p>
            </div>
          </div>

          {/* Explicitly Rejected Features */}
          <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-6 my-6">
            <h3 className="text-xl font-bold mb-4">Features We've Explicitly Rejected</h3>
            <div className="space-y-4">
              <div>
                <p className="font-semibold mb-2">Plugin System</p>
                <p className="text-sm text-slate-600 dark:text-slate-300">Users can modify the source. A plugin system adds complexity and constraints. The absence of a plugin API means users can change anything.</p>
              </div>
              <div>
                <p className="font-semibold mb-2">Memory Search Ranking</p>
                <p className="text-sm text-slate-600 dark:text-slate-300">We return results ordered by modification time. The LLM decides what's relevant in its current context.</p>
              </div>
              <div>
                <p className="font-semibold mb-2">Automatic Backup</p>
                <p className="text-sm text-slate-600 dark:text-slate-300">Backup is a user concern. We don't make assumptions about your backup strategy.</p>
              </div>
              <div>
                <p className="font-semibold mb-2">Rate Limiting</p>
                <p className="text-sm text-slate-600 dark:text-slate-300">This is a local tool. If you want to abuse your own system, that's your choice.</p>
              </div>
              <div>
                <p className="font-semibold mb-2">Telemetry</p>
                <p className="text-sm text-slate-600 dark:text-slate-300">Not even anonymous. Not even opt-in. The absence of telemetry is a feature.</p>
              </div>
              <div>
                <p className="font-semibold mb-2">Update Checker</p>
                <p className="text-sm text-slate-600 dark:text-slate-300">Users can check for updates when they want to. We don't phone home.</p>
              </div>
            </div>
          </div>

          {/* Power of Absence */}
          <div className="bg-blue-50 dark:bg-slate-800 border border-blue-200 dark:border-blue-900 rounded-lg p-6">
            <h3 className="text-xl font-bold mb-4">The Power of Absence</h3>
            <p className="mb-4">Each missing feature creates space for:</p>
            <ul className="space-y-2">
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>User customization without fighting defaults</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>LLM decision-making without constraints</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Simple debugging without abstraction layers</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Clear understanding without documentation</span>
              </li>
              <li className="flex gap-3">
                <span className="text-blue-500 flex-shrink-0">•</span>
                <span>Trust without privacy concerns</span>
              </li>
            </ul>
            <p className="mt-4 italic font-semibold text-center">The absence IS the feature.</p>
          </div>
        </section>

        {/* Design Principles Section */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold mb-6">Design Principles</h2>

          <div className="grid gap-6">
            <div className="border-l-4 border-blue-500 pl-6 py-4">
              <h3 className="text-lg font-bold mb-2">Transparency First</h3>
              <p className="text-slate-700 dark:text-slate-300">
                Every operation is inspectable. Thinking sessions show all revisions and branches. Search results include similarity scores. Graph relationships are queryable SQL. All content is readable markdown.
              </p>
            </div>

            <div className="border-l-4 border-blue-500 pl-6 py-4">
              <h3 className="text-lg font-bold mb-2">Lazy Evaluation</h3>
              <p className="text-slate-700 dark:text-slate-300">
                Nothing is pre-computed or pre-structured. The graph builds from natural WikiLink usage. Embeddings generate on demand. Relationships emerge from repetition. Structure follows function.
              </p>
            </div>

            <div className="border-l-4 border-blue-500 pl-6 py-4">
              <h3 className="text-lg font-bold mb-2">Composable Tools</h3>
              <p className="text-slate-700 dark:text-slate-300">
                Each tool does one thing well. WriteMemory creates markdown with WikiLinks. SequentialThinking enables iterative reasoning with revision. BuildContext traverses the graph from concepts. Workflow orchestrates tool composition.
              </p>
            </div>

            <div className="border-l-4 border-blue-500 pl-6 py-4">
              <h3 className="text-lg font-bold mb-2">Files as Source of Truth</h3>
              <p className="text-slate-700 dark:text-slate-300">
                Not a black box database. Direct file access for debugging. Git-compatible for version control. Obsidian-compatible for human editing. Standard markdown for portability.
              </p>
            </div>

            <div className="border-l-4 border-blue-500 pl-6 py-4">
              <h3 className="text-lg font-bold mb-2">Complete Observability</h3>
              <p className="text-slate-700 dark:text-slate-300">
                Watch AI thinking in real-time. Every reasoning session, every revision, every branch—all queryable, all transparent. No black boxes. No hidden decisions.
              </p>
            </div>
          </div>
        </section>

        {/* Footer Message */}
        <div className="text-center py-8 border-t border-slate-200 dark:border-slate-700">
          <p className="text-slate-600 dark:text-slate-400 italic">
            間 The space between thoughts becomes knowledge<br />
            ∴ Knowledge compounds into wisdom
          </p>
        </div>
      </article>
    </div>
  );
}
