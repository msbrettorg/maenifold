// T-SITE-001.23: Homepage redesign — factory narrative, architecture, built-with proof
// Server component — editorial homepage centered on what maenifold produces
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
            Context engineering infrastructure for AI agents.
          </p>
          <p style={{ color: 'var(--text-secondary)', fontSize: '1.125rem', lineHeight: 1.8, margin: 0 }}>
            A local reasoning substrate that other systems build on. It manages a knowledge graph,
            produces specialized roles and workflows, and sharpens itself through use.
            Domain-specific plugins layer on top.
          </p>
        </div>
      </section>

      {/* 2. Graph break */}
      <figure className="graph-break">
        <img
          src="/graph.jpeg"
          alt="Knowledge graph visualization — 1,623 concepts connected by 58,851 links, colored by connection strength"
        />
      </figure>

      {/* 3. The flywheel */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Seed. Create. Run. Compound.</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '2.5rem' }}>
            Infrastructure that extends itself. Agents create workflows. Workflows produce
            knowledge. Knowledge sharpens the graph. The graph makes better agents.
          </p>
        </div>
        <dl className="flywheel-steps">
          <div className="flywheel-step">
            <dt>Seed the graph</dt>
            <dd>
              Run <code>think-tank</code> on any domain&#39;s literature. Parallel agents research
              in waves &mdash; scoping, deep dives, synthesis, peer review. They write memories
              with <code>[[WikiLinks]]</code>. Structure emerges from use &mdash; no schema, no
              ontology, no upfront design.
            </dd>
          </div>
          <div className="flywheel-step">
            <dt>Create specialists</dt>
            <dd>
              <code>role-creation-workflow</code> analyzes your graph and produces constitutional
              role definitions &mdash; experts grounded in your actual knowledge, not generic
              personas. 16 built-in roles. Write more. They persist.
            </dd>
          </div>
          <div className="flywheel-step">
            <dt>Run workflows</dt>
            <dd>
              39 built-in workflows, from deductive reasoning to multi-agent sprints. Schedule
              via cron. They produce structured output &mdash; reports, analyses, roadmaps &mdash;
              while you sleep. Assets are writable from agents, so new workflows are
              agent-generated skills.
            </dd>
          </div>
          <div className="flywheel-step">
            <dt>Watch it compound</dt>
            <dd>
              Decay removes noise. Consolidation strengthens signal. Community detection discovers
              reasoning domains. Every run makes the next one smarter. The graph isn&#39;t storage
              &mdash; it&#39;s the reasoning substrate that shapes what context is retrieved and how
              it can be extended.
            </dd>
          </div>
        </dl>
      </section>

      {/* 4. Architecture — primitive layers */}
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
          15+ RAG techniques supported natively or via composition &mdash; Knowledge Graph RAG,
          HyDE, FLARE, RAG-Fusion, Self-RAG, multi-hop traversal. These are infrastructure
          primitives: small, composable, unix-pipeable. Complexity emerges from
          scripting them together. The full technique matrix is in <a href="/docs">the docs</a>.
        </p>
      </section>

      {/* 5. Test-time adaptation */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Test-Time Adaptation</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
            Roles and colors change how the model reasons over the same graph &mdash; not
            just tone. Same data, different cognitive strategy.
          </p>
        </div>
        <div className="adaptation-grid">
          <div className="adaptation-card">
            <div className="adaptation-label" style={{ color: '#4CAF50' }}>Green Hat</div>
            <div className="adaptation-mode">Creative</div>
            <p>Knowledge mutation engine, cross-domain pattern transfer, emergent role
              discovery, &ldquo;what if&rdquo; scenarios.</p>
          </div>
          <div className="adaptation-card">
            <div className="adaptation-label" style={{ color: '#FFC107' }}>Yellow Hat</div>
            <div className="adaptation-mode">Value</div>
            <p>Cost governance, enterprise readiness, knowledge leverage, value affirmation.</p>
          </div>
          <div className="adaptation-card">
            <div className="adaptation-label" style={{ color: '#F44336' }}>Black Hat</div>
            <div className="adaptation-mode">Critical</div>
            <p>Identity crisis &amp; scope creep, unvalidated claims, graph integrity risks,
              maintenance burden.</p>
          </div>
        </div>
        <p style={{ color: 'var(--text-secondary)', marginTop: '2rem', fontSize: '0.9375rem', maxWidth: '60ch' }}>
          All three analyses ran against the same 52-concept graph. 7 thinking colors, 16 roles,
          12 perspectives &mdash; cognitive modes that change what the model retrieves, questions,
          and prioritizes. Not prompt decoration. Infrastructure for how agents reason.
        </p>
      </section>

      {/* 6. What ships */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>What ships</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
            Four plugins. Each layers on the substrate for a different domain.
          </p>
        </div>
        <div className="built-with-list">
          <article className="built-with-item">
            <div className="built-with-meta">Base</div>
            <h3>plugin-maenifold</h3>
            <p>
              The substrate. Knowledge graph, 25+ MCP tools, 39 workflows, 16 roles,
              7 thinking colors. Graph-of-thought priming at session start. [[WikiLink]]
              context injection on every task. Memory lifecycle &mdash; decay, consolidation,
              community detection.
            </p>
            <div className="built-with-stats">
              <span>25+ tools</span>
              <span className="built-with-divider">/</span>
              <span>39 workflows</span>
              <span className="built-with-divider">/</span>
              <span>16 roles</span>
            </div>
          </article>

          <article className="built-with-item">
            <div className="built-with-meta">Dogfooding</div>
            <h3>plugin-product-team</h3>
            <p>
              Used to build maenifold itself. Product manager orchestrator, 4 specialized
              agents (SWE, researcher, red-team, blue-team), TDD pipeline with red-team/blue-team
              validation. Sprint planning, requirements traceability, parallel task execution.
            </p>
            <div className="built-with-stats">
              <span>4 agents + PM skill</span>
              <span className="built-with-divider">/</span>
              <span>TDD pipeline</span>
            </div>
          </article>

          <article className="built-with-item">
            <div className="built-with-meta">Microsoft FinOps Toolkit</div>
            <h3>plugin-finops-toolkit</h3>
            <p>
              The official Claude Code plugin
              for <a href="https://github.com/microsoft/finops-toolkit" target="_blank" rel="noopener noreferrer">Microsoft&rsquo;s FinOps Toolkit</a>.
              4 agents (CFO, FinOps practitioner, database query, hubs), 4 commands,
              2 skills, an output style, and 17 KQL queries. Layers on maenifold &mdash;
              the agents inherit the knowledge graph, the reasoning workflows, the memory lifecycle.
            </p>
            <div className="built-with-stats">
              <span>4 agents + 2 skills + 17 queries</span>
              <span className="built-with-divider">/</span>
              <span><a href="https://github.com/microsoft/finops-toolkit/pull/2013" target="_blank" rel="noopener noreferrer">microsoft/finops-toolkit#2013</a></span>
            </div>
          </article>

          <article className="built-with-item">
            <div className="built-with-meta">Azure ISV capacity</div>
            <h3>plugin-capacity-management</h3>
            <p>
              Purpose-built for SaaS ISVs operating in Azure. Quota operations, capacity
              reservations, zone management, estate-level governance. Companion material
              to training offered by Microsoft&rsquo;s Customer Success Unit.
              Documentation site, training decks, lab modules, and scripts.
            </p>
            <div className="built-with-stats">
              <span>Agent + skill + docs site</span>
              <span className="built-with-divider">/</span>
              <span><a href="https://github.com/msbrettorg/azcapman" target="_blank" rel="noopener noreferrer">github.com/msbrettorg/azcapman</a></span>
            </div>
          </article>
        </div>
      </section>

      {/* 7. Install */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Install</h2>
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

      {/* 8. Explore */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
          <h2>Explore</h2>
          <nav aria-label="Site navigation" style={{ display: 'flex', flexWrap: 'wrap', gap: '1.5rem', marginTop: '1rem' }}>
            <a href="/docs" style={{ fontSize: '1rem' }}>Docs</a>
            <a href="/plugins" style={{ fontSize: '1rem' }}>Plugins</a>
            <a href="/tools" style={{ fontSize: '1rem' }}>Tools</a>
            <a href="/workflows" style={{ fontSize: '1rem' }}>Workflows</a>
            <a href="/getting-started" style={{ fontSize: '1rem' }}>Getting Started</a>
            <a
              href="https://github.com/msbrettorg/maenifold"
              target="_blank"
              rel="noopener noreferrer"
              style={{ fontSize: '1rem' }}
            >
              GitHub
            </a>
          </nav>
        </div>
      </section>

    </main>
  );
}
