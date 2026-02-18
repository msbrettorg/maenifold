// T-SITE-001.10: RTM FR-15.12

import type { Metadata } from 'next'
import Link from 'next/link'
import { notFound } from 'next/navigation'
import { getAllWorkflows, getWorkflowById } from '@/lib/assets'
import { WorkflowActions } from './WorkflowActions'

export async function generateStaticParams() {
  return getAllWorkflows().map((w) => ({ id: w.id }))
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ id: string }>
}): Promise<Metadata> {
  const { id } = await params
  const workflow = getWorkflowById(id)

  if (!workflow) {
    return {
      title: 'Workflow Not Found | Maenifold',
      description: 'The requested workflow could not be found.',
    }
  }

  return {
    title: `${workflow.name} | Maenifold Workflows`,
    description: workflow.description,
    openGraph: {
      title: workflow.name,
      description: workflow.description,
    },
  }
}

export default async function WorkflowPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const workflow = getWorkflowById(id)

  if (!workflow) {
    notFound()
  }

  return (
    <div style={{ backgroundColor: 'var(--color-bg)', color: 'var(--color-text)', minHeight: '100vh' }}>
      {/* Breadcrumbs */}
      <div style={{ maxWidth: 900, margin: '0 auto', padding: '24px 16px 0', fontSize: 14, color: 'var(--color-text-secondary)' }}>
        <Link href="/" style={{ color: 'var(--color-text-secondary)', textDecoration: 'none' }}>
          Home
        </Link>
        <span style={{ margin: '0 8px' }}>/</span>
        <Link href="/workflows" style={{ color: 'var(--color-text-secondary)', textDecoration: 'none' }}>
          Workflows
        </Link>
        <span style={{ margin: '0 8px' }}>/</span>
        <span style={{ color: 'var(--color-text)' }}>{workflow.name}</span>
      </div>

      <div style={{ maxWidth: 900, margin: '0 auto', padding: '48px 16px' }}>
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 24, flexWrap: 'wrap' }}>
          <div>
            <h1 style={{ fontSize: 32, fontWeight: 600, marginBottom: 16 }}>
              <span style={{ marginRight: 8 }}>{workflow.emoji}</span>
              {workflow.name}
            </h1>
            <p style={{ fontSize: 18, color: 'var(--color-text-secondary)', marginBottom: 24, lineHeight: 1.75 }}>
              {workflow.description}
            </p>
          </div>
          <WorkflowActions workflowId={workflow.id} workflowName={workflow.name} />
        </div>

        {/* Steps */}
        <div style={{ marginTop: 40 }}>
          <h2 style={{ fontSize: 24, fontWeight: 600, marginBottom: 16 }}>Steps</h2>
          <ol style={{ listStyle: 'none', padding: 0, margin: 0, display: 'grid', gap: 16 }}>
            {workflow.steps?.map((step, index) => (
              <li
                key={step.id}
                style={{
                  padding: 16,
                  borderRadius: 12,
                  border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
                  backgroundColor: 'var(--color-bg-surface)',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 16 }}>
                  <div>
                    <div style={{ fontSize: 13, color: 'var(--color-text-secondary)', marginBottom: 4 }}>
                      Step {index + 1}
                    </div>
                    <div style={{ fontSize: 18, fontWeight: 600 }}>
                      {step.name}
                    </div>
                    <div style={{ color: 'var(--color-text-secondary)', marginTop: 8, whiteSpace: 'pre-wrap', lineHeight: 1.75 }}>
                      {step.description}
                    </div>
                  </div>
                  {step.requiresEnhancedThinking && (
                    <span style={{
                      fontSize: 12,
                      fontWeight: 600,
                      padding: '4px 8px',
                      borderRadius: 6,
                      backgroundColor: 'var(--color-accent-muted)',
                      color: 'var(--color-accent)',
                      flexShrink: 0,
                    }}>
                      Enhanced thinking
                    </span>
                  )}
                </div>
              </li>
            ))}
          </ol>
        </div>

        {/* Triggers */}
        {workflow.triggers && workflow.triggers.length > 0 && (
          <div style={{ marginTop: 48 }}>
            <h2 style={{ fontSize: 24, fontWeight: 600, marginBottom: 16 }}>Triggers</h2>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
              {workflow.triggers.map((t) => (
                <span
                  key={t}
                  style={{
                    fontSize: 12,
                    padding: '4px 10px',
                    borderRadius: 999,
                    backgroundColor: 'var(--color-bg-surface)',
                    border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
                    color: 'var(--color-text)',
                  }}
                >
                  {t}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Navigation */}
        <div style={{
          marginTop: 48,
          paddingTop: 32,
          borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
          display: 'flex',
          justifyContent: 'space-between',
        }}>
          <Link
            href="/workflows"
            style={{
              padding: '8px 16px',
              backgroundColor: 'var(--color-bg-surface)',
              borderRadius: 8,
              color: 'var(--color-text)',
              textDecoration: 'none',
            }}
          >
            &larr; Back to Workflows
          </Link>
          <a
            href="#top"
            style={{
              padding: '8px 16px',
              backgroundColor: 'var(--color-bg-surface)',
              borderRadius: 8,
              color: 'var(--color-text)',
              textDecoration: 'none',
            }}
          >
            &uarr; Back to Top
          </a>
        </div>
      </div>
    </div>
  )
}
