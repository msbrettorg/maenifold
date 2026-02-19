// T-SITE-001.17: RTM FR-15.12, FR-15.21 — Workflow detail page (static, data-driven)
import fs from 'fs';
import path from 'path';
import type { Metadata } from 'next';
import { renderMarkdown } from '@/lib/markdown';

// --- Types ---

interface ToolHint {
  reason: string;
  priority: string;
  estimated_thoughts?: number;
  focus_areas?: string[];
}

interface StepMetadata {
  toolHints?: Record<string, ToolHint>;
  reasoning_effort?: string;
  stop_conditions?: string[];
  guardrails?: Record<string, string>;
  next_actions?: string[];
  time_budget?: string;
}

interface WorkflowStep {
  id: string;
  name: string;
  description?: string;
  requiresEnhancedThinking?: boolean;
  metadata?: StepMetadata;
}

interface Workflow {
  id: string;
  name: string;
  emoji: string;
  description?: string;
  triggers?: string[];
  enhancedThinkingEnabled?: boolean;
  steps: WorkflowStep[];
}

// --- Data loading ---

const workflowsDir = path.join(process.cwd(), '..', 'src', 'assets', 'workflows');

function loadWorkflow(id: string): Workflow | null {
  const filePath = path.join(workflowsDir, `${id}.json`);
  if (!fs.existsSync(filePath)) return null;
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8')) as Workflow;
  return {
    ...data,
    steps: Array.isArray(data.steps) ? data.steps : [],
  };
}

// --- Static params for `output: 'export'` ---

export function generateStaticParams(): { id: string }[] {
  const files = fs.readdirSync(workflowsDir).filter((f) => f.endsWith('.json'));
  return files.map((f) => ({ id: f.replace('.json', '') }));
}

// --- Metadata ---

export async function generateMetadata({ params }: { params: Promise<{ id: string }> }): Promise<Metadata> {
  const { id } = await params;
  const workflow = loadWorkflow(id);
  if (!workflow) {
    return { title: 'Workflow Not Found — maenifold' };
  }
  return {
    title: `${workflow.name} — maenifold`,
    description: workflow.description ?? `${workflow.name} workflow for maenifold.`,
  };
}

// --- Page component ---

export default async function WorkflowDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const workflow = loadWorkflow(id);

  if (!workflow) {
    return (
      <main style={{ maxWidth: '72ch', marginInline: 'auto', padding: '4rem 1.5rem' }}>
        <a href="/workflows" style={{ fontSize: '0.875rem', color: 'var(--accent)' }}>
          &larr; Workflows
        </a>
        <h1 style={{ marginTop: '2rem' }}>Workflow not found</h1>
        <p style={{ color: 'var(--text-secondary)' }}>
          No workflow with id &ldquo;{id}&rdquo; exists.
        </p>
      </main>
    );
  }

  // Pre-render all step descriptions at build time
  const renderedDescriptions = await Promise.all(
    workflow.steps.map((step) =>
      step.description ? renderMarkdown(step.description) : Promise.resolve(null)
    )
  );

  const stepCount = workflow.steps.length;

  return (
    <main style={{ maxWidth: '72ch', marginInline: 'auto', padding: '4rem 1.5rem' }}>
      <style>{`
        .trigger-chip {
          display: inline-block;
          font-family: ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace;
          font-size: 0.75rem;
          padding: 0.2em 0.55em;
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 4px;
          line-height: 1.5;
          white-space: nowrap;
        }
        .tool-chip {
          display: inline-block;
          font-family: ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace;
          font-size: 0.7rem;
          padding: 0.15em 0.45em;
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 4px;
          line-height: 1.5;
          white-space: nowrap;
        }
        .step-card {
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 8px;
          padding: 1.5rem;
        }
        .enhanced-badge {
          display: inline-flex;
          align-items: center;
          gap: 0.3rem;
          font-size: 0.75rem;
          color: var(--text-secondary);
          background: var(--bg-hover, var(--bg-surface));
          border: 1px solid var(--border);
          border-radius: 4px;
          padding: 0.2em 0.6em;
          margin-top: 0.75rem;
        }
        .effort-badge {
          display: inline-block;
          font-size: 0.7rem;
          font-family: ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace;
          padding: 0.15em 0.5em;
          border-radius: 4px;
          background: var(--accent-muted);
          color: var(--accent);
          border: 1px solid var(--accent-muted);
        }
        .guardrail-note {
          font-size: 0.8125rem;
          color: var(--text-secondary);
          margin-top: 0.75rem;
          padding: 0.5rem 0.75rem;
          border-left: 2px solid var(--border);
          line-height: 1.55;
        }
      `}</style>

      {/* Back link */}
      <nav style={{ marginBottom: '2rem' }}>
        <a href="/workflows" style={{ fontSize: '0.875rem' }}>
          &larr; Workflows
        </a>
      </nav>

      {/* Header */}
      <header style={{ marginBottom: '2.5rem' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: '0.625rem', marginBottom: '0.5rem' }}>
          <span style={{ fontSize: '2rem', lineHeight: 1 }} aria-hidden="true">
            {workflow.emoji}
          </span>
          <h1 style={{ margin: 0 }}>{workflow.name}</h1>
        </div>
        <p style={{ margin: 0, color: 'var(--text-secondary)', fontSize: '0.875rem' }}>
          {stepCount} step{stepCount !== 1 ? 's' : ''}
        </p>
      </header>

      {/* Description */}
      {workflow.description && (
        <section style={{ marginBottom: '2rem' }}>
          <p style={{ margin: 0, lineHeight: 1.75 }}>{workflow.description}</p>
        </section>
      )}

      {/* Triggers */}
      {workflow.triggers && workflow.triggers.length > 0 && (
        <section style={{ marginBottom: '2.5rem' }}>
          <h2 style={{ fontSize: '0.8125rem', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--text-secondary)', marginBottom: '0.625rem' }}>
            Triggers
          </h2>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.375rem' }}>
            {workflow.triggers.map((trigger) => (
              <span key={trigger} className="trigger-chip">
                {trigger}
              </span>
            ))}
          </div>
        </section>
      )}

      {/* Steps */}
      <section>
        <h2 style={{ fontSize: '0.8125rem', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--text-secondary)', marginBottom: '1rem' }}>
          Steps
        </h2>
        <ol style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          {workflow.steps.map((step, index) => {
            const descHtml = renderedDescriptions[index];
            const meta = step.metadata;
            const toolHintEntries = meta?.toolHints ? Object.entries(meta.toolHints) : [];
            const guardrailEntries = meta?.guardrails ? Object.entries(meta.guardrails) : [];

            return (
              <li key={step.id} className="step-card">
                {/* Step number + name */}
                <div style={{ display: 'flex', alignItems: 'baseline', gap: '0.625rem', marginBottom: descHtml ? '0.875rem' : 0 }}>
                  <span
                    style={{
                      fontSize: '0.75rem',
                      fontFamily: 'ui-monospace, monospace',
                      color: 'var(--text-secondary)',
                      flexShrink: 0,
                      minWidth: '1.5rem',
                    }}
                  >
                    {index + 1}.
                  </span>
                  <h3 style={{ margin: 0, fontSize: '1rem', fontWeight: 600 }}>{step.name}</h3>
                </div>

                {/* Step description — rendered markdown */}
                {descHtml && (
                  <div
                    className="markdown-content"
                    style={{ fontSize: '0.9375rem', lineHeight: 1.7, marginTop: '0.75rem' }}
                    // biome-ignore lint/security/noDangerouslySetInnerHtml: build-time markdown render
                    dangerouslySetInnerHTML={{ __html: descHtml }}
                  />
                )}

                {/* Enhanced Thinking indicator */}
                {step.requiresEnhancedThinking && (
                  <div className="enhanced-badge" aria-label="Enhanced Thinking required">
                    <span aria-hidden="true">&#129504;</span>
                    Enhanced Thinking
                  </div>
                )}

                {/* Metadata section */}
                {meta && (
                  <div style={{ marginTop: '1rem' }}>
                    {/* Tool hints */}
                    {toolHintEntries.length > 0 && (
                      <div style={{ marginBottom: '0.625rem' }}>
                        <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginRight: '0.5rem' }}>
                          Tools:
                        </span>
                        <span style={{ display: 'inline-flex', flexWrap: 'wrap', gap: '0.3rem' }}>
                          {toolHintEntries.map(([toolName, hint]) => (
                            <span
                              key={toolName}
                              className="tool-chip"
                              title={hint.reason}
                            >
                              {toolName}
                              {hint.priority === 'mandatory' && (
                                <span style={{ color: 'var(--accent)', marginLeft: '0.25em' }}>*</span>
                              )}
                            </span>
                          ))}
                        </span>
                      </div>
                    )}

                    {/* Reasoning effort badge */}
                    {meta.reasoning_effort && (
                      <div style={{ marginBottom: '0.5rem' }}>
                        <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginRight: '0.5rem' }}>
                          Effort:
                        </span>
                        <span className="effort-badge">{meta.reasoning_effort}</span>
                      </div>
                    )}

                    {/* Guardrails note */}
                    {guardrailEntries.length > 0 && (
                      <div className="guardrail-note">
                        <strong style={{ fontSize: '0.75rem', textTransform: 'uppercase', letterSpacing: '0.04em' }}>
                          Guardrails
                        </strong>
                        <ul style={{ margin: '0.25rem 0 0', padding: '0 0 0 1rem', listStyle: 'disc' }}>
                          {guardrailEntries.map(([key, value]) => (
                            <li key={key} style={{ marginBottom: '0.125rem' }}>
                              <span style={{ fontFamily: 'ui-monospace, monospace', fontSize: '0.7rem' }}>{key}</span>
                              {': '}
                              {value}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ol>
      </section>
    </main>
  );
}
