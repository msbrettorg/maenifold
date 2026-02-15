'use client'

import { useMemo, useState } from 'react'
import { CopyButton } from '@/app/components/CopyButton'

interface WorkflowActionsProps {
  workflowId: string
  workflowName: string
}

export function WorkflowActions({ workflowId, workflowName }: WorkflowActionsProps) {
  const runCommand = useMemo(() => {
    return `maenifold --tool Workflow --payload '{\"workflowId\":\"${workflowId}\"}'`
  }, [workflowId])

  const [shareStatus, setShareStatus] = useState<'idle' | 'success' | 'error'>('idle')

  const handleShare = async () => {
    setShareStatus('idle')

    const url = new URL(window.location.href)
    url.searchParams.set('utm_source', 'workflow_share')
    url.searchParams.set('utm_medium', 'share_button')
    url.searchParams.set('utm_campaign', 'workflows')
    const shareUrl = url.toString()

    let method: 'web_share' | 'clipboard' = typeof navigator.share === 'function' ? 'web_share' : 'clipboard'

    try {
      if (method === 'web_share') {
        await navigator.share({
          title: `Maenifold Workflow: ${workflowName}`,
          url: shareUrl,
        })
      } else {
        await navigator.clipboard.writeText(shareUrl)
      }

      setShareStatus('success')
    } catch (error) {
      // User cancellation in Web Share API should not show an error state.
      if (method === 'web_share' && error instanceof DOMException && error.name === 'AbortError') {
        setShareStatus('idle')
      } else {
        // Best-effort fallback: if sharing fails, try clipboard.
        if (method === 'web_share') {
          method = 'clipboard'
          try {
            await navigator.clipboard.writeText(shareUrl)
            setShareStatus('success')
          } catch {
            setShareStatus('error')
          }
        } else {
          setShareStatus('error')
        }
      }
    }
  }

  return (
    <div className="flex flex-col sm:flex-row gap-3">
      <button
        onClick={handleShare}
        className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold transition-colors"
      >
        {shareStatus === 'success' ? 'Shared!' : shareStatus === 'error' ? 'Share failed' : 'Share'}
      </button>

      <div className="flex items-center gap-2">
        <CopyButton
          text={runCommand}
          label="Copy run command"
          copiedLabel="Command copied!"
          className="px-4 py-2 rounded-lg bg-slate-100 dark:bg-slate-900 hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-900 dark:text-white font-semibold transition-colors"
        />
      </div>
    </div>
  )
}
