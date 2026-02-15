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
    <div className="min-h-screen bg-white dark:bg-slate-950 text-slate-900 dark:text-white">
      <div className="max-w-4xl mx-auto px-4 py-6 text-sm text-slate-600 dark:text-slate-400">
        <Link href="/" className="hover:text-blue-600 dark:hover:text-blue-400">
          Home
        </Link>
        <span className="mx-2">/</span>
        <Link href="/workflows" className="hover:text-blue-600 dark:hover:text-blue-400">
          Workflows
        </Link>
        <span className="mx-2">/</span>
        <span className="text-slate-900 dark:text-white">{workflow.name}</span>
      </div>

      <div className="max-w-4xl mx-auto px-4 py-12">
        <div className="flex items-start justify-between gap-6 flex-col sm:flex-row">
          <div>
            <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">
              <span className="mr-2">{workflow.emoji}</span>
              {workflow.name}
            </h1>
            <p className="text-lg text-slate-700 dark:text-slate-300 mb-6">
              {workflow.description}
            </p>
          </div>
          <WorkflowActions workflowId={workflow.id} workflowName={workflow.name} />
        </div>

        <div className="mt-10">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">Steps</h2>
          <ol className="space-y-4">
            {workflow.steps?.map((step, index) => (
              <li
                key={step.id}
                className="p-4 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900"
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="text-sm text-slate-500 dark:text-slate-400 mb-1">
                      Step {index + 1}
                    </div>
                    <div className="text-lg font-semibold text-slate-900 dark:text-white">
                      {step.name}
                    </div>
                    <div className="text-slate-700 dark:text-slate-300 mt-2 whitespace-pre-wrap">
                      {step.description}
                    </div>
                  </div>
                  {step.requiresEnhancedThinking && (
                    <span className="text-xs font-semibold px-2 py-1 rounded bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 shrink-0">
                      Enhanced thinking
                    </span>
                  )}
                </div>
              </li>
            ))}
          </ol>
        </div>

        {workflow.triggers && workflow.triggers.length > 0 && (
          <div className="mt-12">
            <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">Triggers</h2>
            <div className="flex flex-wrap gap-2">
              {workflow.triggers.map((t) => (
                <span
                  key={t}
                  className="text-xs px-2 py-1 rounded bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-300"
                >
                  {t}
                </span>
              ))}
            </div>
          </div>
        )}

        <div className="mt-12 pt-8 border-t border-slate-200 dark:border-slate-800 flex justify-between">
          <Link
            href="/workflows"
            className="px-4 py-2 bg-slate-100 dark:bg-slate-900 hover:bg-slate-200 dark:hover:bg-slate-800 rounded-lg transition-colors"
          >
            ← Back to Workflows
          </Link>
          <a
            href="#top"
            className="px-4 py-2 bg-slate-100 dark:bg-slate-900 hover:bg-slate-200 dark:hover:bg-slate-800 rounded-lg transition-colors"
          >
            ↑ Back to Top
          </a>
        </div>
      </div>
    </div>
  )
}

