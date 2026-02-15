'use client'

import { useState } from 'react'

interface CopyButtonProps {
  text: string
  label?: string
  copiedLabel?: string
  onCopied?: () => void | Promise<void>
  className?: string
}

export function CopyButton({ text, label = 'Copy', copiedLabel = 'Copied!', onCopied, className = '' }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    await onCopied?.()
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <button
      onClick={handleCopy}
      className={className}
    >
      {copied ? copiedLabel : label}
    </button>
  )
}
