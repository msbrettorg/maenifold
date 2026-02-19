// T-SITE-001.6: RTM FR-15.2, FR-15.3, FR-15.4, FR-15.5, FR-15.7, FR-15.8, FR-15.14, FR-15.20, FR-15.24, FR-15.30
// T-SITE-001.19: Updated tagline — added "a knowledge graph and"
// Server component — all content sourced directly from README.md
import { CopyButton } from './components/CopyButton';

// Source: README.md lines 22-28
const INSTALL_CODE = `# Homebrew (macOS/Linux)
brew install msbrettorg/tap/maenifold

# Manual — download from GitHub Releases
# https://github.com/msbrettorg/maenifold/releases/latest`;

// Source: README.md lines 40-46
const MCP_CONFIG = `{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}`;

// Source: README.md lines 33-36
const CLI_EXAMPLES = `maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'`;

export default async function Home() {

  return (
    <main>

        {/* 1. Hero — FR-15.2 */}
        <section className="prose-width section-gap" style={{ padding: '0 1rem' }}>
          <h1 style={{ marginBottom: '1rem' }}>maenifold</h1>
          <p style={{ fontSize: '1.25rem', lineHeight: '1.6', marginBottom: '0.75rem' }}>
            Context engineering infrastructure for AI agents.
          </p>
          <p style={{ color: 'var(--text-secondary)', fontSize: '1.0625rem', lineHeight: '1.75', margin: 0 }}>
            Point it at any domain&#39;s literature, and it builds a knowledge graph and specialized experts that live on your
            machine, work offline, and get smarter with every use.
          </p>
        </section>

        {/* 2. How It Works — FR-15.5 */}
        <section className="section-gap" style={{ padding: '0 1rem' }}>
          <div className="prose-width">
            <h2>How It Works</h2>
            <p>Seed the graph. Keep the experts. Watch it compound.</p>
            <ul style={{ paddingLeft: '1.5rem', lineHeight: '2' }}>
              <li><strong><code>{'[[WikiLinks]]'}</code></strong> &mdash; lightweight concept identifiers, not payloads</li>
              <li><strong>Hybrid search</strong> &mdash; semantic vectors + full-text with RRF fusion</li>
              <li><strong>Knowledge graph</strong> &mdash; lazy construction, structure emerges from use</li>
              <li><strong>Sequential thinking</strong> &mdash; multi-step reasoning with revision, branching, persistence</li>
              <li><strong>35+ workflows</strong> &mdash; deductive reasoning to multi-agent sprints</li>
              <li><strong>Memory lifecycle</strong> &mdash; decay, consolidation, repair modeled on cognitive neuroscience</li>
              <li><strong>16 roles, 7 thinking colors, 12 perspectives</strong> &mdash; composable cognitive assets</li>
              <li><strong>Community detection</strong> &mdash; Louvain algorithm identifies reasoning domains during sync</li>
              <li><strong>Decay weighting</strong> &mdash; ACT-R power-law recency bias across search, context, and similarity</li>
              <li><strong>Graph-of-thought priming</strong> &mdash; hook system injects clustered concept maps at session start</li>
            </ul>
            <p style={{ color: 'var(--text-secondary)', marginTop: '1.5rem' }}>
              Six layers: WikiLinks &rarr; Graph &rarr; Search &rarr; Session State &rarr; Reasoning &rarr; Orchestration.
            </p>
          </div>
        </section>

        {/* 3. Installation — FR-15.3, FR-15.20 */}
        <section className="section-gap" style={{ padding: '0 1rem' }}>
          <div className="prose-width" style={{ marginBottom: '1rem' }}>
            <h2>Installation</h2>
          </div>
          <div className="code-block code-width" style={{ position: 'relative' }}>
            <pre><code>{INSTALL_CODE}</code></pre>
            <CopyButton text={INSTALL_CODE} />
          </div>
        </section>

        {/* 4. MCP Configuration — FR-15.4 */}
        <section className="section-gap" style={{ padding: '0 1rem' }}>
          <div className="prose-width" style={{ marginBottom: '1rem' }}>
            <h2>MCP Configuration</h2>
            <p style={{ color: 'var(--text-secondary)' }}>
              Claude Code, Claude Desktop, Codex, and compatible MCP clients:
            </p>
          </div>
          <div className="code-block code-width" style={{ position: 'relative' }}>
            <pre><code>{MCP_CONFIG}</code></pre>
            <CopyButton text={MCP_CONFIG} />
          </div>
        </section>

        {/* 5. CLI Examples — FR-15.5 */}
        <section className="section-gap" style={{ padding: '0 1rem' }}>
          <div className="prose-width" style={{ marginBottom: '1rem' }}>
            <h2>CLI Examples</h2>
          </div>
          <div className="code-block code-width" style={{ position: 'relative' }}>
            <pre><code>{CLI_EXAMPLES}</code></pre>
            <CopyButton text={CLI_EXAMPLES} />
          </div>
        </section>

        {/* 6. Explore — FR-15.8 */}
        <section className="section-gap" style={{ padding: '0 1rem' }}>
          <div className="prose-width">
            <h2>Explore</h2>
            <nav aria-label="Site navigation" style={{ display: 'flex', flexWrap: 'wrap', gap: '1.5rem', marginTop: '1rem' }}>
              <a href="/docs" style={{ fontSize: '1rem' }}>Docs</a>
              <a href="/plugins" style={{ fontSize: '1rem' }}>Plugins</a>
              <a href="/tools" style={{ fontSize: '1rem' }}>Tools</a>
              <a href="/workflows" style={{ fontSize: '1rem' }}>Workflows</a>
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
