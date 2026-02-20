// T-SITE-001.23: Homepage redesign — benefit-forward narrative, proof domains, guided next steps
// Server component — editorial homepage centered on what maenifold does for agents
import { CopyButton } from './components/CopyButton';

const INSTALL = `# Install the binary
brew install msbrettorg/tap/maenifold

# Add the plugin marketplace and install the base plugin
claude plugin marketplace add msbrettorg/maenifold
claude plugin install maenifold@maenifold-marketplace`;

export default function Home() {
  return (
    <main>

      {/* 1. Hero */}
      <section className="section-container" style={{ paddingTop: '5rem', paddingBottom: '0' }}>
        <div style={{ maxWidth: '55ch' }}>
          <h1 style={{
            fontSize: '3.5rem',
            fontWeight: 400,
            letterSpacing: '-0.02em',
            lineHeight: 1.1,
            color: 'var(--accent)',
            marginBottom: '1.5rem',
          }}>
            maenifold
          </h1>
          <p style={{ fontSize: '1.375rem', lineHeight: 1.6, marginBottom: '1rem' }}>
            One graph. Every agent. Knowledge that compounds.
          </p>
          <p style={{ color: 'var(--text-secondary)', fontSize: '1.125rem', lineHeight: 1.8, margin: 0 }}>
            Agents think in chains of thought. Maenifold captures the important bits
            as <code>[[WikiLinks]]</code>, builds a graph of just those concepts and how
            they relate, and feeds it back into the context window. Memory for humans.
            Graph for agents. One graph, every AI tool on your machine.
          </p>
          <div className="badge-row">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="https://img.shields.io/github/stars/msbrettorg/maenifold?style=flat&color=6AABB3" alt="GitHub stars" />
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="https://img.shields.io/github/v/release/msbrettorg/maenifold?style=flat&color=6AABB3" alt="Latest release" />
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="https://img.shields.io/badge/license-MIT-6AABB3?style=flat" alt="MIT license" />
          </div>
        </div>
      </section>

      {/* 2. Graph break */}
      <figure className="graph-break">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src="/graph.jpeg"
          alt="Knowledge graph visualization — 1,623 concepts connected by 58,851 links, colored by connection strength"
          fetchPriority="high"
          loading="eager"
        />
        <figcaption>1,623 concepts. 58,851 links. Built by agents.</figcaption>
      </figure>

      {/* 3. Problem statement */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>The problem</h2>
          <p style={{ color: 'var(--text-secondary)', lineHeight: 1.8 }}>
            Agent sessions are stateless. Everything learned disappears when the session ends.
            Agents across tools can&rsquo;t share what they know &mdash; Claude Code doesn&rsquo;t know
            what Copilot learned. RAG retrieves documents and hopes for the best. It
            doesn&rsquo;t understand relationships. The context window starts empty every time.
          </p>
        </div>
      </section>

      {/* 4. How it works */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>How it works</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '0.5rem', fontSize: '1.125rem' }}>
            Seed. Create. Run. Compound.
          </p>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '2.5rem' }}>
            Infrastructure that extends itself. Agents create workflows. Workflows produce
            knowledge. Knowledge sharpens the graph. The graph makes better agents.
          </p>
        </div>
        <dl className="flywheel-steps">
          <div className="flywheel-step">
            <dt>Seed the graph</dt>
            <dd>
              Point agents at any domain&rsquo;s literature. They research in parallel, tag concepts
              with <code>[[WikiLinks]]</code>, and structure emerges &mdash; no schema required.
            </dd>
          </div>
          <div className="flywheel-step">
            <dt>Create specialists</dt>
            <dd>
              Generate roles and agents grounded in your actual knowledge. 16 built-in.
              Write more. They persist.
            </dd>
          </div>
          <div className="flywheel-step">
            <dt>Run workflows</dt>
            <dd>
              39 built-in workflows. Schedule via cron. Agents produce structured
              output &mdash; reports, analyses, roadmaps &mdash; while you sleep.
            </dd>
          </div>
          <div className="flywheel-step">
            <dt>Watch it compound</dt>
            <dd>
              Noise decays. Signal strengthens. Reasoning domains emerge automatically.
              Every run makes the next one smarter.
            </dd>
          </div>
        </dl>
      </section>

      {/* 5. Proof domains */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Built with maenifold</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
            Three production domains. Zero overlap in subject matter. Same infrastructure underneath.
          </p>
        </div>
        <div className="built-with-list">
          <article className="built-with-item">
            <div className="built-with-meta">Microsoft FinOps Toolkit</div>
            <h3>plugin-finops-toolkit</h3>
            <p>
              The official Claude Code plugin for Microsoft&rsquo;s FinOps Toolkit. 4 autonomous
              agents analyze cloud cost data, run KQL queries against Azure Data Explorer,
              and produce FinOps reports on a cron schedule &mdash; no human in the loop.
            </p>
            <div className="built-with-stats">
              <span>4 agents + 2 skills + 17 queries</span>
            </div>
            <div className="built-with-downloads">
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/plugin-finops-toolkit.zip" aria-label="Download plugin-finops-toolkit plugin">plugin</a>
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/skill-finops-toolkit.zip" aria-label="Download finops-toolkit skill">skill: finops-toolkit</a>
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/skill-azure-cost-management.zip" aria-label="Download azure-cost-management skill">skill: azure-cost-management</a>
            </div>
          </article>

          <article className="built-with-item">
            <div className="built-with-meta">Azure ISV capacity</div>
            <h3>plugin-capacity-management</h3>
            <p>
              Purpose-built for SaaS ISVs operating in Azure. An agent manages quota operations,
              capacity reservations, and zone-level governance across subscription estates.
              Companion material to training offered by Microsoft&rsquo;s Customer Success Unit.
            </p>
            <div className="built-with-stats">
              <span>Agent + skill + docs site</span>
            </div>
            <div className="built-with-downloads">
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/plugin-capacity-management.zip" aria-label="Download plugin-capacity-management plugin">plugin</a>
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/skill-azure-capacity-management.zip" aria-label="Download azure-capacity-management skill">skill</a>
            </div>
          </article>

          <article className="built-with-item">
            <div className="built-with-meta">Dogfooding</div>
            <h3>plugin-product-team</h3>
            <p>
              Used to build maenifold itself. A product manager orchestrates 4 specialized
              agents &mdash; SWE, researcher, red-team, blue-team &mdash; through sprint planning,
              TDD validation, and parallel task execution.
            </p>
            <div className="built-with-stats">
              <span>4 agents + PM skill</span>
              <span className="built-with-divider">/</span>
              <span>TDD pipeline</span>
            </div>
            <div className="built-with-downloads">
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/plugin-product-team.zip" aria-label="Download plugin-product-team plugin">plugin</a>
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/skill-product-manager.zip" aria-label="Download product-manager skill">skill</a>
            </div>
          </article>

          <article className="built-with-item">
            <div className="built-with-meta">Base</div>
            <h3>plugin-maenifold</h3>
            <p>
              The substrate. Knowledge graph, 25+ MCP tools, 39 workflows, 16 roles,
              7 thinking colors. Graph-of-thought priming at session start. <code>[[WikiLink]]</code> context
              injection on every task. Memory lifecycle &mdash; decay, consolidation,
              community detection.
            </p>
            <div className="built-with-stats">
              <span>25+ tools</span>
              <span className="built-with-divider">/</span>
              <span>39 workflows</span>
              <span className="built-with-divider">/</span>
              <span>16 roles</span>
            </div>
            <div className="built-with-downloads">
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/plugin-maenifold.zip" aria-label="Download plugin-maenifold plugin">plugin</a>
              <a href="https://github.com/msbrettorg/maenifold/releases/latest/download/skill-maenifold.zip" aria-label="Download maenifold skill">skill</a>
            </div>
          </article>
        </div>
      </section>

      {/* 6. Architecture — primitive layers */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Architecture</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
            Six layers of composable primitives. Complexity emerges from composition.
          </p>
        </div>
        <div className="layers-table-wrap">
          <table className="layers-table">
            <thead>
              <tr>
                <th>Layer</th>
                <th>Primitives</th>
                <th>Purpose</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td><strong>Memory</strong></td>
                <td><code>WriteMemory</code> <code>ReadMemory</code> <code>EditMemory</code> <code>DeleteMemory</code> <code>MoveMemory</code> <code>Sync</code></td>
                <td>Persist knowledge with <code>[[WikiLinks]]</code></td>
              </tr>
              <tr>
                <td><strong>Graph</strong></td>
                <td><code>SearchMemories</code> <code>BuildContext</code> <code>FindSimilarConcepts</code> <code>Visualize</code></td>
                <td>Query and traverse knowledge</td>
              </tr>
              <tr>
                <td><strong>Session</strong></td>
                <td><code>RecentActivity</code> <code>AssumptionLedger</code> <code>ListMemories</code></td>
                <td>Track work, state, and uncertainty</td>
              </tr>
              <tr>
                <td><strong>Persona</strong></td>
                <td><code>Adopt</code></td>
                <td>Roles, colors, and perspectives</td>
              </tr>
              <tr>
                <td><strong>Reasoning</strong></td>
                <td><code>SequentialThinking</code></td>
                <td>Multi-step thought with revision and branching</td>
              </tr>
              <tr>
                <td><strong>Orchestration</strong></td>
                <td><code>Workflow</code></td>
                <td>State machines over steps and tools</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p style={{ color: 'var(--text-secondary)', marginTop: '1.5rem', fontSize: '0.9375rem', maxWidth: '60ch' }}>
          15+ retrieval techniques supported natively or via composition. The full
          technique matrix is in <a href="/docs">the docs</a>.
        </p>
      </section>

      {/* 7. Install */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Install</h2>
          <div className="badge-row" style={{ marginTop: '0', marginBottom: '1.5rem' }}>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="https://img.shields.io/github/stars/msbrettorg/maenifold?style=flat&color=6AABB3" alt="GitHub stars" />
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="https://img.shields.io/github/v/release/msbrettorg/maenifold?style=flat&color=6AABB3" alt="Latest release" />
          </div>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>
            Install the binary, add the marketplace, install the plugin. The infrastructure
            activates &mdash; graph-of-thought priming, [[WikiLink]] context injection, 25+ tools,
            39 workflows, 16 roles.
          </p>
        </div>
        <div className="code-block" style={{ position: 'relative' }}>
          <pre><code>{INSTALL}</code></pre>
          <CopyButton text={INSTALL} />
        </div>
        <p style={{ color: 'var(--text-secondary)', marginTop: '1.5rem', fontSize: '0.9375rem' }}>
          <a href="/getting-started">Full setup guide</a> &mdash; MCP configuration, CLI examples,
          product team plugin.
        </p>
      </section>

      {/* 8. Next steps */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Next steps</h2>
          <div className="next-steps">
            <a href="/getting-started">
              <h3>Getting Started</h3>
              <p>Install, configure, write your first memory.</p>
            </a>
            <a href="/docs">
              <h3>Docs</h3>
              <p>Architecture, concepts, tool reference.</p>
            </a>
            <a href="https://github.com/msbrettorg/maenifold" target="_blank" rel="noopener noreferrer">
              <h3>GitHub</h3>
              <p>Source, issues, releases.</p>
            </a>
          </div>
          <nav aria-label="Additional navigation" style={{ marginTop: '1.5rem', color: 'var(--text-secondary)', fontSize: '0.9375rem' }}>
            Also explore: <a href="/plugins">Plugins</a> &middot; <a href="/tools">Tools</a> &middot; <a href="/workflows">Workflows</a> &middot; <a href="/adapt">Adapt</a>
          </nav>
        </div>
      </section>

    </main>
  );
}
