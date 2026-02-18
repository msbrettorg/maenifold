// T-SITE-001.22 (FR-15.30)
'use client'

import { useEffect, useRef } from 'react'

const CLIPBOARD_SVG =
  '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>'
const CHECK_SVG =
  '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>'

export function MarkdownContent({ html, className = '' }: { html: string; className?: string }) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const container = ref.current
    if (!container) {
      return
    }

    const pres = container.querySelectorAll('pre')
    const cleanups: (() => void)[] = []

    pres.forEach((pre) => {
      pre.style.position = 'relative'

      // Add right padding to prevent text overlapping the button
      const currentPadding = getComputedStyle(pre).paddingRight
      const paddingValue = parseInt(currentPadding, 10) || 16
      if (paddingValue < 36) {
        pre.style.paddingRight = '36px'
      }

      const btn = document.createElement('button')
      btn.type = 'button'
      btn.title = 'Copy'
      btn.setAttribute('aria-label', 'Copy')
      btn.innerHTML = CLIPBOARD_SVG
      Object.assign(btn.style, {
        position: 'absolute',
        top: '8px',
        right: '8px',
        width: '20px',
        height: '20px',
        display: 'grid',
        placeItems: 'center',
        color: 'var(--color-text-secondary)',
        background: 'transparent',
        border: '0',
        padding: '0',
        cursor: 'pointer',
        transition: 'color 120ms ease',
      })

      btn.addEventListener('mouseenter', () => {
        btn.style.color = 'var(--color-accent)'
      })
      btn.addEventListener('mouseleave', () => {
        if (btn.dataset.copied !== 'true') {
          btn.style.color = 'var(--color-text-secondary)'
        }
      })

      btn.addEventListener('click', async () => {
        const text = pre.textContent ?? ''
        try {
          await navigator.clipboard.writeText(text)
        } catch {
          return // clipboard unavailable — degrade silently
        }
        btn.innerHTML = CHECK_SVG
        btn.style.color = 'var(--color-accent)'
        btn.title = 'Copied!'
        btn.setAttribute('aria-label', 'Copied!')
        btn.dataset.copied = 'true'
        setTimeout(() => {
          btn.innerHTML = CLIPBOARD_SVG
          btn.style.color = 'var(--color-text-secondary)'
          btn.title = 'Copy'
          btn.setAttribute('aria-label', 'Copy')
          btn.dataset.copied = 'false'
        }, 2000)
      })

      pre.appendChild(btn)
      cleanups.push(() => btn.remove())
    })

    return () => cleanups.forEach((fn) => fn())
  }, [html])

  return <div ref={ref} className={className} dangerouslySetInnerHTML={{ __html: html }} />
}
