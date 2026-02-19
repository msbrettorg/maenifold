// T-SITE-001.13: RTM FR-15.30
'use client';

import { useState } from 'react';

export function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <button
      onClick={handleCopy}
      aria-label="Copy code to clipboard"
      style={{
        position: 'absolute',
        top: '0.5rem',
        right: '0.5rem',
        padding: '0.25rem 0.5rem',
        fontSize: '12px',
        color: copied ? 'var(--accent)' : 'var(--text-secondary)',
        background: 'var(--bg-hover)',
        border: '1px solid var(--border)',
        borderRadius: '4px',
        cursor: 'pointer',
      }}
    >
      {copied ? 'Copied!' : 'Copy'}
    </button>
  );
}
