import { Metadata } from 'next'
import Link from 'next/link'
import { getAllToolSlugs, loadToolData, extractSections } from '@/lib/tools'
import { MarkdownContent } from '@/app/components/MarkdownContent'

// T-SITE-001.9: RTM FR-15.11

/**
 * Generate static parameters for all tool pages.
 * Enables Next.js to pre-generate all pages at build time.
 */
export async function generateStaticParams() {
  const slugs = getAllToolSlugs()
  return slugs.map((slug) => ({
    slug,
  }))
}

/**
 * Generate metadata for each tool page (for SEO).
 */
export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>
}): Promise<Metadata> {
  try {
    const { slug } = await params
    const toolData = await loadToolData(slug)
    return {
      title: `${toolData.title} | Maenifold Tools`,
      description: toolData.description,
      openGraph: {
        title: toolData.title,
        description: toolData.description,
      },
    }
  } catch {
    return {
      title: 'Tool Not Found | Maenifold',
      description: 'The requested tool documentation could not be found.',
    }
  }
}

/**
 * Individual tool page component.
 */
export default async function ToolPage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params
  let toolData
  try {
    toolData = await loadToolData(slug)
  } catch {
    return (
      <div style={{ backgroundColor: 'var(--color-bg)', color: 'var(--color-text)', minHeight: '100vh' }}>
        <div style={{ maxWidth: 900, margin: '0 auto', padding: '48px 16px' }}>
          <h1 style={{ fontSize: 32, fontWeight: 600, marginBottom: 16 }}>Tool Not Found</h1>
          <p style={{ color: 'var(--color-text-secondary)', marginBottom: 24 }}>
            The tool &ldquo;{slug}&rdquo; could not be found.
          </p>
          <Link href="/tools" style={{ color: 'var(--color-accent)', textDecoration: 'none' }}>
            &larr; Back to Tools
          </Link>
        </div>
      </div>
    )
  }

  const sections = extractSections(toolData.htmlContent)

  return (
    <div style={{ backgroundColor: 'var(--color-bg)', color: 'var(--color-text)', minHeight: '100vh' }}>
      {/* Breadcrumbs */}
      <div style={{ maxWidth: 900, margin: '0 auto', padding: '24px 16px 0', fontSize: 14, color: 'var(--color-text-secondary)' }}>
        <Link href="/" style={{ color: 'var(--color-text-secondary)', textDecoration: 'none' }}>
          Home
        </Link>
        <span style={{ margin: '0 8px' }}>/</span>
        <Link href="/tools" style={{ color: 'var(--color-text-secondary)', textDecoration: 'none' }}>
          Tools
        </Link>
        <span style={{ margin: '0 8px' }}>/</span>
        <span style={{ color: 'var(--color-text)' }}>{toolData.title}</span>
      </div>

      <div style={{ maxWidth: 900, margin: '0 auto', padding: '48px 16px' }}>
        {/* Header */}
        <h1 style={{ fontSize: 32, fontWeight: 600, marginBottom: 32 }}>
          {toolData.title}
        </h1>

        {/* Content */}
        <article>
          <MarkdownContent html={styleHtml(toolData.htmlContent)} className="markdown-content" />
        </article>

        {/* Navigation */}
        <div style={{
          marginTop: 48,
          paddingTop: 32,
          borderTop: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
          display: 'flex',
          justifyContent: 'space-between',
        }}>
          <Link
            href="/tools"
            style={{
              padding: '8px 16px',
              backgroundColor: 'var(--color-bg-surface)',
              borderRadius: 8,
              color: 'var(--color-text)',
              textDecoration: 'none',
            }}
          >
            &larr; Back to Tools
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

      {/* Sidebar - Table of Contents (sticky, wide viewports only) */}
      {sections.length > 0 && (
        <style>{`
          @media (min-width: 1200px) {
            .tool-sidebar {
              display: block !important;
              position: fixed;
              top: 100px;
              right: max(16px, calc((100vw - 900px) / 2 - 260px));
              width: 220px;
            }
          }
        `}</style>
      )}
      {sections.length > 0 && (
        <aside className="tool-sidebar" style={{ display: 'none' }}>
          <nav style={{
            backgroundColor: 'var(--color-bg-surface)',
            padding: 24,
            borderRadius: 12,
            border: '1px solid color-mix(in srgb, var(--color-text) 12%, transparent)',
          }}>
            <h3 style={{ fontSize: 12, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: 12, color: 'var(--color-text-secondary)' }}>
              Contents
            </h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0, fontSize: 13 }}>
              {sections.map((section) => (
                <li key={section.id} style={{ marginLeft: `${(section.level - 2) * 16}px`, marginTop: 6 }}>
                  <a
                    href={`#${section.id}`}
                    style={{ color: 'var(--color-accent)', textDecoration: 'none' }}
                  >
                    {section.title}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        </aside>
      )}
    </div>
  )
}

/**
 * Apply design system styles to markdown HTML.
 * Uses ([\s\S]*?) to match heading content that may contain nested HTML tags (e.g. <code>).
 */
function styleHtml(html: string): string {
  const idCounts = new Map<string, number>()

  function deduplicateId(baseId: string): string {
    const count = idCounts.get(baseId) || 0
    const id = count > 0 ? `${baseId}-${count}` : baseId
    idCounts.set(baseId, count + 1)
    return id
  }

  // H2 headings — use [\s\S]*? to handle nested HTML like <code>
  html = html.replace(/<h2([^>]*)>([\s\S]*?)<\/h2>/g, (_match, attrs: string, content: string) => {
    const text = content.replace(/<[^>]*>/g, '')
    const baseId = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')

    const id = deduplicateId(baseId)
    return `<h2${attrs} id="${id}" style="font-size:24px;font-weight:600;margin-top:32px;margin-bottom:16px;scroll-margin-top:96px">${content}</h2>`
  })

  // H3 headings
  html = html.replace(/<h3([^>]*)>([\s\S]*?)<\/h3>/g, (_match, attrs: string, content: string) => {
    const text = content.replace(/<[^>]*>/g, '')
    const baseId = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')

    const id = deduplicateId(baseId)
    return `<h3${attrs} id="${id}" style="font-size:20px;font-weight:600;margin-top:24px;margin-bottom:12px;scroll-margin-top:96px">${content}</h3>`
  })

  // Paragraphs — use [\s\S]*? to handle inline markup
  html = html.replace(/<p>([\s\S]*?)<\/p>/g, '<p style="margin:16px 0;line-height:1.75;color:var(--color-text)">$1</p>')

  // Lists
  html = html.replace(/<ul>/g, '<ul style="list-style:disc;padding-left:24px;margin:16px 0;color:var(--color-text)">')
  html = html.replace(/<ol>/g, '<ol style="list-style:decimal;padding-left:24px;margin:16px 0;color:var(--color-text)">')
  html = html.replace(/<li>/g, '<li style="margin:8px 0">')

  // Tables
  html = html.replace(/<table>/g, '<table style="width:100%;margin:24px 0;border-collapse:collapse">')
  html = html.replace(/<thead>/g, '<thead style="background:var(--color-bg-surface)">')
  html = html.replace(/<th>/g, '<th style="border:1px solid color-mix(in srgb,var(--color-text) 12%,transparent);padding:8px 12px;text-align:left;font-weight:600">')
  html = html.replace(/<td>/g, '<td style="border:1px solid color-mix(in srgb,var(--color-text) 12%,transparent);padding:8px 12px">')

  // Blockquotes
  html = html.replace(/<blockquote>/g, '<blockquote style="border-left:4px solid var(--color-accent);padding-left:16px;margin:16px 0;color:var(--color-text-secondary);font-style:italic">')

  // Strong
  html = html.replace(/<strong>/g, '<strong style="font-weight:600;color:var(--color-text)">')

  // Links — don't re-style already-styled links
  html = html.replace(
    /<a(?!\s+style)([^>]*)href="([^"]+)"([^>]*)>/g,
    '<a$1href="$2"$3 style="color:var(--color-accent);text-decoration:none">'
  )

  return html
}
