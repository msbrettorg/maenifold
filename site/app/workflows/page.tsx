// T-SITE-001.10

import type { Metadata } from 'next'
import Link from 'next/link'

import { getAllWorkflows } from '../../lib/assets'

export const metadata: Metadata = {
  title: 'Workflows',
}

export default function WorkflowsPage() {
  // Build-time data load (static export friendly).
  const workflows = getAllWorkflows()

  return (
    <div className="mx-auto px-4 py-20" style={{ maxWidth: '900px' }}>
      <header>
        <h1
          style={{
            color: 'var(--color-text)',
            fontSize: '2.25rem',
            letterSpacing: '-0.02em',
            lineHeight: 1.1,
          }}
        >
          Workflows ({workflows.length})
        </h1>

        <p
          style={{
            color: 'var(--color-text-secondary)',
            marginTop: '0.75rem',
            lineHeight: 1.75,
          }}
        >
          A catalog of all built-in workflows shipped with maenifold.
        </p>
      </header>

      <section style={{ marginTop: '2.5rem' }}>
        {workflows.length === 0 ? (
          <div
            style={{
              color: 'var(--color-text-secondary)',
              border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
              borderRadius: '12px',
              padding: '1rem',
            }}
          >
            No workflows found.
          </div>
        ) : (
          <ul style={{ display: 'grid', gap: '0.75rem', listStyle: 'none', padding: 0, margin: 0 }}>
            {workflows.map((workflow) => {
              const stepCount = workflow.steps?.length ?? 0
              const triggers = workflow.triggers ?? []

              return (
                <li
                  key={workflow.id}
                  style={{
                    border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
                    borderRadius: '12px',
                    padding: '1rem',
                    background: 'var(--color-bg-surface)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: '0.75rem' }}>
                    <span aria-hidden="true" style={{ fontSize: '1.25rem' }}>
                      {workflow.emoji}
                    </span>
                    <Link
                      href={`/workflows/${workflow.id}`}
                      style={{
                        color: 'var(--color-accent)',
                        textDecoration: 'none',
                        fontSize: '1.125rem',
                        fontWeight: 600,
                        letterSpacing: '-0.01em',
                      }}
                    >
                      {workflow.name}
                    </Link>
                  </div>

                  <p
                    style={{
                      color: 'var(--color-text-secondary)',
                      marginTop: '0.5rem',
                      lineHeight: 1.75,
                    }}
                  >
                    {workflow.description}
                  </p>

                  <div
                    style={{
                      display: 'flex',
                      flexWrap: 'wrap',
                      gap: '0.5rem',
                      marginTop: '0.75rem',
                      alignItems: 'center',
                    }}
                  >
                    <span
                      style={{
                        color: 'var(--color-text-secondary)',
                        fontSize: '0.875rem',
                      }}
                    >
                      {stepCount} step{stepCount === 1 ? '' : 's'}
                    </span>

                    {triggers.length > 0 ? (
                      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.375rem' }}>
                        <span
                          style={{
                            color: 'var(--color-text-secondary)',
                            fontSize: '0.875rem',
                          }}
                        >
                          Triggers:
                        </span>
                        {triggers.map((trigger) => (
                          <span
                            key={trigger}
                            style={{
                              fontSize: '0.75rem',
                              color: 'var(--color-text)',
                              border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
                              borderRadius: '999px',
                              padding: '0.125rem 0.5rem',
                            }}
                          >
                            {trigger}
                          </span>
                        ))}
                      </div>
                    ) : null}
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </section>
    </div>
  )
}
