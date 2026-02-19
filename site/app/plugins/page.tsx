// T-SITE-001.8b: RTM FR-15.10 — /plugins page redesign: card-based layout
import { CopyButton } from '@/app/components/CopyButton';

export const metadata = {
  title: 'Plugins — maenifold',
  description: 'Claude Code plugin setup and configuration for maenifold.',
};

const mcpConfig = `{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}`;

const installBase = `claude plugin add /path/to/integrations/claude-code/plugin-maenifold`;
const installProductTeam = `claude plugin add /path/to/integrations/claude-code/plugin-product-team`;

interface PluginComponent {
  type: string;
  name: string;
}

interface PluginDef {
  id: string;
  label: string;
  badge: string;
  description: string;
  note?: string;
  components: PluginComponent[];
  installCommand: string;
}

const plugins: PluginDef[] = [
  {
    id: 'plugin-maenifold',
    label: 'plugin-maenifold',
    badge: 'Base',
    description: 'Knowledge graph and reasoning infrastructure',
    components: [
      { type: 'Hook', name: 'SessionStart' },
      { type: 'Hook', name: 'PreToolUse' },
      { type: 'Hook', name: 'SubagentStop' },
      { type: 'Skill', name: 'maenifold' },
    ],
    installCommand: installBase,
  },
  {
    id: 'plugin-product-team',
    label: 'plugin-product-team',
    badge: 'Opinionated',
    description: 'Multi-agent product team with TDD pipeline',
    note: 'Requires plugin-maenifold.',
    components: [
      { type: 'Agent', name: 'swe' },
      { type: 'Agent', name: 'researcher' },
      { type: 'Agent', name: 'red-team' },
      { type: 'Agent', name: 'blue-team' },
      { type: 'Skill', name: 'product-manager' },
    ],
    installCommand: installProductTeam,
  },
];

export default function PluginsPage() {
  return (
    <main style={{ maxWidth: '1100px', marginInline: 'auto', padding: '4rem 1.5rem' }}>
      <style>{`
        .plugin-card {
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 8px;
          padding: 2rem;
          transition: border-color 0.15s ease;
        }
        .plugin-card:hover {
          border-color: var(--accent-muted);
        }
        .plugin-badge {
          display: inline-block;
          font-size: 0.7rem;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          padding: 0.15em 0.55em;
          border-radius: 4px;
          background: var(--accent-muted);
          color: var(--accent);
          border: 1px solid var(--accent-muted);
          vertical-align: middle;
          margin-left: 0.5rem;
        }
        .component-chip {
          display: inline-flex;
          align-items: baseline;
          gap: 0.35em;
          font-family: ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace;
          font-size: 0.8125rem;
          padding: 0.2em 0.6em;
          background: var(--bg-hover, var(--bg-surface));
          border: 1px solid var(--border);
          border-radius: 4px;
          line-height: 1.5;
          white-space: nowrap;
        }
        .chip-type {
          font-size: 0.65rem;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          color: var(--text-secondary);
          font-weight: 600;
        }
        .plugin-section-label {
          font-size: 0.75rem;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          color: var(--text-secondary);
          margin-bottom: 0.625rem;
        }
      `}</style>

      {/* Page header */}
      <header style={{ marginBottom: '2.5rem' }}>
        <h1 style={{ marginBottom: '0.5rem' }}>Plugins</h1>
        <p style={{ color: 'var(--text-secondary)', margin: '0 0 0.25rem', maxWidth: '60ch', lineHeight: 1.6 }}>
          Two-layer plugin architecture for Claude Code. The base plugin provides the knowledge
          graph and reasoning tools. The product team plugin adds opinionated agents and workflows
          on top.
        </p>
        <p style={{ margin: 0, fontSize: '0.875rem', color: 'var(--text-secondary)', fontFamily: 'ui-monospace, monospace' }}>
          2 plugins
        </p>
      </header>

      {/* Plugin cards — stacked vertically */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', marginBottom: '3rem' }}>
        {plugins.map((plugin) => (
          <article key={plugin.id} className="plugin-card">
            {/* Card header */}
            <div style={{ marginBottom: '0.75rem' }}>
              <h2 style={{ margin: 0, fontSize: '1.125rem', fontWeight: 600, display: 'inline', fontFamily: 'ui-monospace, monospace' }}>
                {plugin.label}
              </h2>
              <span className="plugin-badge">{plugin.badge}</span>
            </div>

            {/* Description */}
            <p style={{ margin: '0 0 1.25rem', fontSize: '0.9375rem', color: 'var(--text-secondary)', lineHeight: 1.55 }}>
              {plugin.description}
              {plugin.note && (
                <> <span style={{ fontStyle: 'italic' }}>{plugin.note}</span></>
              )}
            </p>

            {/* Components */}
            <div style={{ marginBottom: '1.5rem' }}>
              <p className="plugin-section-label">Components</p>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.375rem' }}>
                {plugin.components.map((comp) => (
                  <span key={`${comp.type}-${comp.name}`} className="component-chip">
                    <span className="chip-type">{comp.type}</span>
                    {comp.name}
                  </span>
                ))}
              </div>
            </div>

            {/* Install command */}
            <div>
              <p className="plugin-section-label">Installation</p>
              <div className="code-block" style={{ position: 'relative', margin: 0 }}>
                <CopyButton text={plugin.installCommand} />
                <pre>
                  <code>{plugin.installCommand}</code>
                </pre>
              </div>
            </div>
          </article>
        ))}
      </div>

      {/* MCP Configuration section */}
      <section>
        <h2 style={{ fontSize: '1.25rem', fontWeight: 600, marginBottom: '0.5rem' }}>
          MCP Configuration
        </h2>
        <p style={{ color: 'var(--text-secondary)', margin: '0 0 1rem', lineHeight: 1.6, maxWidth: '60ch' }}>
          Add the following to your MCP client configuration to connect Claude Code to the
          maenifold MCP server:
        </p>
        <div className="code-block code-width" style={{ position: 'relative' }}>
          <CopyButton text={mcpConfig} />
          <pre>
            <code>{mcpConfig}</code>
          </pre>
        </div>
      </section>
    </main>
  );
}
