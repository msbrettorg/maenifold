// T-SITE-001.6, T-SITE-001.17–21: RTM FR-15.2–FR-15.8, FR-15.14, FR-15.20, FR-15.24, FR-15.30

import Image from 'next/image';
import Link from 'next/link';
import type { CSSProperties } from 'react';
import { CopyButton } from './components/CopyButton';
import { renderMermaidToSvg } from '@/lib/markdown';

// 6-layer cognitive stack diagram from integrations/claude-code/plugin-maenifold/README.md
const LAYER_DIAGRAM = `graph TB
    subgraph "Layer 6: Orchestration"
        Orchestration[Workflow<br/>Multi-step processes<br/>Nested workflows]
    end

    subgraph "Layer 5: Reasoning"
        Reasoning[Sequential Thinking<br/>Branching, revision<br/>Multi-session persistence]
    end

    subgraph "Layer 4: Persona"
        Persona[Adopt<br/>Roles, colors, perspectives<br/>Conditioned reasoning]
    end

    subgraph "Layer 3: Session"
        Session[Recent Activity<br/>Assumption Ledger<br/>State tracking]
    end

    subgraph "Layer 2: Memory + Graph"
        Memory[Write/Read/Search/Edit<br/>BuildContext, FindSimilar<br/>Persist & Query]
    end

    subgraph "Layer 1: Concepts"
        Concepts["WikiLinks<br/>Atomic units<br/>Graph nodes"]
    end

    Orchestration -->|invokes| Reasoning
    Reasoning -->|conditions| Persona
    Persona -->|tracks| Session
    Session -->|queries| Memory
    Memory -->|built from| Concepts

    style Orchestration fill:#0969DA
    style Reasoning fill:#0969DA
    style Persona fill:#0969DA
    style Session fill:#0969DA
    style Memory fill:#0969DA
    style Concepts fill:#0969DA`;

// MCP config from README.md lines 40-46 — must match exactly
const MCP_CONFIG = `{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}`;

// CLI examples from README.md lines 32-36 — must match exactly
const CLI_EXAMPLES = [
  `maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'`,
  `maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'`,
  `maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'`,
];

export default async function Home() {
  // FR-15.6: Render the 6-layer diagram at build time via mmdc
  let layerSvg = '';
  try {
    layerSvg = await renderMermaidToSvg(LAYER_DIAGRAM);
  } catch {
    // Build-time rendering failed — degrade gracefully to no diagram
  }

  const sectionGapPx = 80;

  const pageStyle: CSSProperties = {
    backgroundColor: 'var(--color-bg)',
    color: 'var(--color-text)',
    padding: '64px 24px',
  };

  const centeredColumnStyle: CSSProperties = {
    margin: '0 auto',
    maxWidth: '72ch',
    lineHeight: 1.75,
  };

  const sectionStyle: CSSProperties = {
    marginTop: `${sectionGapPx}px`,
  };

  const cardStyle: CSSProperties = {
    backgroundColor: 'var(--color-bg-surface)',
    border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
    borderRadius: 12,
    padding: 20,
  };

  const codeWrapStyle: CSSProperties = {
    margin: '16px auto 0',
    maxWidth: 900,
    position: 'relative',
  };

  const preStyle: CSSProperties = {
    backgroundColor: 'var(--color-code-bg)',
    border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
    borderRadius: 12,
    padding: '16px 36px 16px 16px',
    overflowX: 'auto',
    fontSize: 14,
    lineHeight: 1.6,
    margin: 0,
  };

  const linkStyle: CSSProperties = {
    color: 'var(--color-accent)',
    textDecoration: 'none',
  };

  return (
    <div style={pageStyle}>
      <div style={centeredColumnStyle}>
        {/* 1) Hero — FR-15.2 */}
        <section>
          <h1 style={{ margin: 0, fontSize: 48, letterSpacing: '-0.02em' }}>maenifold</h1>
          <p style={{ margin: '12px 0 0', fontSize: 18 }}>
            Context engineering infrastructure for AI agents.
          </p>
          <p style={{ margin: '16px 0 0' }}>
            Point it at any domain&apos;s literature, and it builds specialized experts that live on your machine,
            work offline, and get smarter with every use.
          </p>
        </section>

        {/* 2) Install — FR-15.3, FR-15.20 */}
        <section style={sectionStyle}>
          <h2 style={{ margin: 0, fontSize: 24 }}>Install</h2>
          <div style={{ marginTop: 16, ...cardStyle }}>
            <div>
              <strong>macOS / Linux</strong>
              <div style={codeWrapStyle}>
                <pre style={preStyle}>
                  <code>brew install msbrettorg/tap/maenifold</code>
                </pre>
                <CopyButton text="brew install msbrettorg/tap/maenifold" />
              </div>
            </div>

            <div style={{ marginTop: 24 }}>
              <strong>Windows</strong>
              <div style={{ marginTop: 8 }}>
                <a
                  href="https://github.com/msbrettorg/maenifold/releases/latest"
                  rel="noreferrer"
                  target="_blank"
                  style={linkStyle}
                >
                  Download the latest release
                </a>
              </div>
            </div>
          </div>
        </section>

        {/* 3) MCP Configuration — FR-15.4, FR-15.30 */}
        <section style={sectionStyle}>
          <h2 style={{ margin: 0, fontSize: 24 }}>MCP Configuration</h2>
          <div style={codeWrapStyle}>
            <pre style={preStyle}>
              <code>{MCP_CONFIG}</code>
            </pre>
            <CopyButton text={MCP_CONFIG} />
          </div>
        </section>

        {/* 4) CLI Examples — FR-15.5, FR-15.30 */}
        <section style={sectionStyle}>
          <h2 style={{ margin: 0, fontSize: 24 }}>CLI Examples</h2>

          {CLI_EXAMPLES.map((example, i) => (
            <div key={i} style={codeWrapStyle}>
              <pre style={preStyle}>
                <code>{example}</code>
              </pre>
              <CopyButton text={example} />
            </div>
          ))}
        </section>

        {/* 5) 6-Layer Cognitive Stack — FR-15.6 */}
        {layerSvg && (
          <section style={sectionStyle}>
            <h2 style={{ margin: 0, fontSize: 24 }}>Architecture</h2>
            <p style={{ margin: '12px 0 0' }}>
              Six layers: WikiLinks → Graph → Search → Session State → Reasoning → Orchestration.
            </p>
            <div
              style={{ marginTop: 16, maxWidth: 900, margin: '16px auto 0' }}
              dangerouslySetInnerHTML={{ __html: layerSvg }}
            />
          </section>
        )}

        {/* 6) Knowledge Graph Screenshot — FR-15.14 */}
        <section style={sectionStyle}>
          <figure style={{ margin: 0 }}>
            <Image
              src="/graph.jpeg"
              alt="Maenifold knowledge graph visualization showing interconnected concepts"
              width={900}
              height={506}
              style={{
                width: '100%',
                height: 'auto',
                borderRadius: 12,
                border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
              }}
            />
            <figcaption style={{ marginTop: 8, fontSize: 14, color: 'var(--color-text-secondary)' }}>
              A real knowledge graph built by maenifold, visualized in Obsidian.
            </figcaption>
          </figure>
        </section>

        {/* 7) Platform Support — FR-15.7, FR-15.24 (matches README exactly) */}
        <section style={sectionStyle}>
          <h2 style={{ margin: 0, fontSize: 24 }}>Platforms</h2>
          <div style={{ marginTop: 16, ...cardStyle, maxWidth: 900 }}>
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <thead>
                <tr>
                  <th style={{ textAlign: 'left', padding: '10px 8px' }}>Platform</th>
                  <th style={{ textAlign: 'left', padding: '10px 8px' }}>Binary</th>
                  <th style={{ textAlign: 'left', padding: '10px 8px' }}>Notes</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    macOS
                  </td>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    <code>osx-arm64</code>, <code>osx-x64</code>
                  </td>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    Apple Silicon or Intel; Homebrew recommended
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    Linux
                  </td>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    <code>linux-x64</code>, <code>linux-arm64</code>
                  </td>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    x64 or ARM64
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    Windows
                  </td>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    <code>win-x64</code>
                  </td>
                  <td style={{ padding: '10px 8px', borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)' }}>
                    x64 only
                  </td>
                </tr>
              </tbody>
            </table>
            <p style={{ margin: '12px 0 0', fontSize: 14, color: 'var(--color-text-secondary)' }}>
              Self-contained (.NET 9.0 bundled). Vector embeddings via ONNX (bundled). No external dependencies.
            </p>
          </div>
        </section>

        {/* 8) Links — FR-15.8 */}
        <section style={sectionStyle}>
          <h2 style={{ margin: 0, fontSize: 24 }}>Links</h2>
          <nav
            aria-label="Primary"
            style={{
              marginTop: 16,
              display: 'flex',
              gap: 16,
              flexWrap: 'wrap',
              alignItems: 'center',
            }}
          >
            <Link href="/docs" style={linkStyle}>
              Docs
            </Link>
            <Link href="/plugins" style={linkStyle}>
              Plugins
            </Link>
            <Link href="/tools" style={linkStyle}>
              Tools
            </Link>
            <Link href="/workflows" style={linkStyle}>
              Workflows
            </Link>
            <a href="https://github.com/msbrettorg/maenifold" rel="noreferrer" target="_blank" style={linkStyle}>
              GitHub
            </a>
          </nav>
        </section>
      </div>
    </div>
  );
}
