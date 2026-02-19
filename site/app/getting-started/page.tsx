// T-SITE-001.22: Getting Started page — moved from homepage
// Server component — install, MCP config, CLI examples
import { CopyButton } from '../components/CopyButton';

export const metadata = {
  title: 'Getting Started — maenifold',
  description: 'Install maenifold, configure MCP, and run your first commands.',
};

const INSTALL_CODE = `# Homebrew (macOS/Linux)
brew install msbrettorg/tap/maenifold

# Manual — download from GitHub Releases
# https://github.com/msbrettorg/maenifold/releases/latest`;

const PLUGIN_INSTALL = `# Base plugin — knowledge graph, reasoning tools, hooks
claude plugin add /path/to/integrations/claude-code/plugin-maenifold

# Product team plugin (optional) — agents, TDD pipeline
claude plugin add /path/to/integrations/claude-code/plugin-product-team`;

const MCP_CONFIG = `{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}`;

const CLI_EXAMPLES = `maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'`;

export default function GettingStartedPage() {
  return (
    <main>
      <section className="section-container" style={{ paddingTop: '4rem' }}>
        <h1 style={{
          fontSize: '2.25rem',
          fontWeight: 400,
          letterSpacing: '-0.015em',
          color: 'var(--accent)',
          marginBottom: '0.5rem',
        }}>
          Getting Started
        </h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: '1.125rem', marginBottom: '3rem', maxWidth: '60ch' }}>
          Install maenifold, add the Claude Code plugin, and start building your knowledge graph.
        </p>
      </section>

      {/* Installation */}
      <section className="section-container section-gap">
        <h2>Installation</h2>
        <div className="code-block" style={{ position: 'relative' }}>
          <pre><code>{INSTALL_CODE}</code></pre>
          <CopyButton text={INSTALL_CODE} />
        </div>
      </section>

      {/* Plugin Setup */}
      <section className="section-container section-gap">
        <h2>Claude Code Plugins</h2>
        <p style={{ color: 'var(--text-secondary)' }}>
          The base plugin activates graph-of-thought priming, [[WikiLink]] context injection,
          and 25+ MCP tools. The product team plugin adds four specialized agents and a TDD pipeline.
        </p>
        <div className="code-block" style={{ position: 'relative' }}>
          <pre><code>{PLUGIN_INSTALL}</code></pre>
          <CopyButton text={PLUGIN_INSTALL} />
        </div>
      </section>

      {/* MCP Configuration */}
      <section className="section-container section-gap">
        <h2>MCP Configuration</h2>
        <p style={{ color: 'var(--text-secondary)' }}>
          For Claude Desktop, Codex, and other MCP clients:
        </p>
        <div className="code-block" style={{ position: 'relative' }}>
          <pre><code>{MCP_CONFIG}</code></pre>
          <CopyButton text={MCP_CONFIG} />
        </div>
      </section>

      {/* CLI Examples */}
      <section className="section-container section-gap">
        <h2>CLI Examples</h2>
        <div className="code-block" style={{ position: 'relative' }}>
          <pre><code>{CLI_EXAMPLES}</code></pre>
          <CopyButton text={CLI_EXAMPLES} />
        </div>
      </section>

      {/* Explore */}
      <section className="section-container section-gap">
        <div style={{ maxWidth: '60ch' }}>
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
