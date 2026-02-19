// T-SITE-001.8: RTM FR-15.10 — /plugins page from plugin manifests and AGENTS.md
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

export default function PluginsPage() {
  return (
    <main className="prose-width" style={{ padding: '3rem 1rem 5rem' }}>
      <h1>Plugins</h1>
      <p>Two-layer plugin architecture for Claude Code.</p>

      {/* plugin-maenifold */}
      <h2>plugin-maenifold (Base)</h2>
      <p>
        <strong>maenifold knowledge graph and reasoning infrastructure.</strong>
      </p>
      <ul>
        <li>MCP server for maenifold tools</li>
        <li>
          Hooks: <code>SessionStart</code>, <code>PreToolUse</code> (Task),{' '}
          <code>SubagentStop</code>
        </li>
        <li>
          Hook script:{' '}
          <code>integrations/claude-code/plugin-maenifold/scripts/hooks.sh</code>
        </li>
        <li>
          Modes: <code>session_start</code>, <code>task_augment</code>,{' '}
          <code>subagent_stop</code>
        </li>
      </ul>
      <p>Installation:</p>
      <div className="code-block code-width" style={{ position: 'relative' }}>
        <CopyButton text={installBase} />
        <pre>
          <code>{installBase}</code>
        </pre>
      </div>

      {/* plugin-product-team */}
      <h2>plugin-product-team (Opinionated)</h2>
      <p>
        <strong>maenifold-enabled product team.</strong> Requires plugin-maenifold
        installed first.
      </p>
      <ul>
        <li>
          Agents: <code>swe</code>, <code>researcher</code>, <code>red-team</code>,{' '}
          <code>blue-team</code>
        </li>
        <li>
          Skills: <code>product-manager</code>
        </li>
      </ul>
      <p>Installation:</p>
      <div className="code-block code-width" style={{ position: 'relative' }}>
        <CopyButton text={installProductTeam} />
        <pre>
          <code>{installProductTeam}</code>
        </pre>
      </div>

      {/* MCP Configuration */}
      <h2>MCP Configuration</h2>
      <p>
        Add the following to your MCP client configuration to connect Claude Code to the
        maenifold MCP server:
      </p>
      <div className="code-block code-width" style={{ position: 'relative' }}>
        <CopyButton text={mcpConfig} />
        <pre>
          <code>{mcpConfig}</code>
        </pre>
      </div>
    </main>
  );
}
