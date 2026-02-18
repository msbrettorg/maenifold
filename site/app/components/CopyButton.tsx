// T-SITE-001.13 (FR-15.30)
'use client'

import { useEffect, useState } from 'react'

export function CopyButton({ text, label = 'Copy', copiedLabel = 'Copied!', onCopied, className = '' }: { text: string; label?: string; copiedLabel?: string; onCopied?: () => void | Promise<void>; className?: string }) {
  const [copied, setCopied] = useState(false)
  useEffect(() => { if (!copied) return; const id = setTimeout(() => setCopied(false), 2000); return () => clearTimeout(id) }, [copied])

  const onClick = async () => { try { await navigator.clipboard.writeText(text); setCopied(true); await onCopied?.() } catch { /* clipboard unavailable — degrade silently */ } }

  const color = copied ? 'var(--color-accent)' : 'var(--color-text-secondary)'
  return (
    <button type="button" onClick={onClick} aria-label={copied ? copiedLabel : label} title={copied ? copiedLabel : label} className={`copyButton ${className}`.trim()}>
      {copied ? (
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M20 6 9 17l-5-5" /></svg>
      ) : (
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>
      )}
      <style jsx>{`.copyButton{position:absolute;top:0;right:0;width:20px;height:20px;display:grid;place-items:center;color:${color};transition:color 120ms ease;background:transparent;border:0;padding:0;cursor:pointer}.copyButton:hover,.copyButton:active{color:var(--color-accent)}`}</style>
    </button>
  )
}
