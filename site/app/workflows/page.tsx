// T-SITE-001.10: RTM FR-15.12, FR-15.21 — /workflows data-driven catalog
// T-SITE-001.18: RTM FR-15.12, FR-15.21 — clickable workflow cards linking to detail pages
import Link from 'next/link';
import fs from 'fs';
import path from 'path';

export const metadata = {
  title: 'Workflows — maenifold',
  description: 'Catalog of all maenifold workflows for structured problem-solving.',
};

interface WorkflowStep {
  id: string;
  name: string;
}

interface Workflow {
  id: string;
  name: string;
  emoji: string;
  description?: string;
  steps: WorkflowStep[];
}

function loadWorkflows(): Workflow[] {
  const workflowsDir = path.join(process.cwd(), '..', 'src', 'assets', 'workflows');
  const files = fs.readdirSync(workflowsDir).filter((f) => f.endsWith('.json'));

  const workflows: Workflow[] = files.map((file) => {
    const raw = fs.readFileSync(path.join(workflowsDir, file), 'utf-8');
    const data = JSON.parse(raw) as Workflow;
    return {
      id: data.id,
      name: data.name,
      emoji: data.emoji,
      description: data.description,
      steps: Array.isArray(data.steps) ? data.steps : [],
    };
  });

  return workflows.sort((a, b) => a.name.localeCompare(b.name));
}

export default function WorkflowsPage() {
  const workflows = loadWorkflows();
  const count = workflows.length;

  return (
    <main style={{ maxWidth: '1100px', marginInline: 'auto', padding: '4rem 1.5rem' }}>
      <style>{`
        .workflow-card {
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 8px;
          padding: 1.5rem;
          transition: border-color 0.15s ease;
          cursor: pointer;
        }
        .workflow-card:hover {
          border-color: var(--accent-muted);
        }
        .workflow-card-link {
          text-decoration: none;
          color: inherit;
          display: block;
        }
      `}</style>

      <header style={{ marginBottom: '2.5rem' }}>
        <h1 style={{ marginBottom: '0.5rem' }}>Workflows</h1>
        <p style={{ color: 'var(--text-secondary)', margin: 0 }}>
          {count} workflows available
        </p>
      </header>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
          gap: '1rem',
        }}
      >
        {workflows.map((workflow) => (
          <Link
            key={workflow.id}
            href={`/workflows/${workflow.id}`}
            className="workflow-card-link"
          >
            <article className="workflow-card">
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.625rem',
                  marginBottom: '0.625rem',
                }}
              >
                <span style={{ fontSize: '1.25rem', lineHeight: 1 }} aria-hidden="true">
                  {workflow.emoji}
                </span>
                <h2 style={{ margin: 0, fontSize: '1rem', fontWeight: 600 }}>
                  {workflow.name}
                </h2>
              </div>

              {workflow.description && (
                <p
                  style={{
                    margin: '0 0 0.75rem',
                    fontSize: '0.875rem',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                  }}
                >
                  {workflow.description}
                </p>
              )}

              <span
                style={{
                  fontSize: '0.75rem',
                  color: 'var(--text-secondary)',
                  fontFamily: 'ui-monospace, monospace',
                }}
              >
                {workflow.steps.length} step{workflow.steps.length !== 1 ? 's' : ''}
              </span>
            </article>
          </Link>
        ))}
      </div>
    </main>
  );
}
