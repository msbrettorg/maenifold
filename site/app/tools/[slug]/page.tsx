import { Metadata } from 'next'
import Link from 'next/link'
import { getAllToolSlugs, loadToolData, extractSections } from '@/lib/tools'
import DOMPurify from 'isomorphic-dompurify'

/**
 * Generate static parameters for all 26 tool pages
 * This enables Next.js to pre-generate all pages at build time
 */
export async function generateStaticParams() {
  const slugs = getAllToolSlugs()
  return slugs.map((slug) => ({
    slug,
  }))
}

/**
 * Generate metadata for each tool page (for SEO)
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
 * Individual tool page component
 */
export default async function ToolPage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params
  let toolData
  try {
    toolData = await loadToolData(slug)
  } catch (error) {
    return (
      <div className="min-h-screen bg-white dark:bg-slate-950">
        <div className="max-w-4xl mx-auto px-4 py-12">
          <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">Tool Not Found</h1>
          <p className="text-slate-600 dark:text-slate-300 mb-6">
            The tool "{slug}" could not be found.
          </p>
          <Link href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline">
            ← Back to Tools
          </Link>
        </div>
      </div>
    )
  }

  const sections = extractSections(toolData.htmlContent)

  return (
    <div className="min-h-screen bg-white dark:bg-slate-950 text-slate-900 dark:text-white">
      {/* Breadcrumbs */}
      <div className="max-w-4xl mx-auto px-4 py-6 text-sm text-slate-600 dark:text-slate-400">
        <Link href="/" className="hover:text-blue-600 dark:hover:text-blue-400">
          Home
        </Link>
        <span className="mx-2">/</span>
        <Link href="/tools" className="hover:text-blue-600 dark:hover:text-blue-400">
          Tools
        </Link>
        <span className="mx-2">/</span>
        <span className="text-slate-900 dark:text-white">{toolData.title}</span>
      </div>

      <div className="max-w-6xl mx-auto px-4 py-12 grid grid-cols-1 lg:grid-cols-4 gap-8">
        {/* Main content */}
        <div className="lg:col-span-3">
          {/* Header */}
          <h1 className="text-4xl font-bold mb-8 text-slate-900 dark:text-white">
            {toolData.title}
          </h1>

          {/* Content */}
          <article className="prose dark:prose-invert max-w-none">
            <MarkdownContent htmlContent={toolData.htmlContent} />
          </article>

          {/* Navigation */}
          <div className="mt-12 pt-8 border-t border-slate-200 dark:border-slate-800 flex justify-between">
            <Link
              href="/tools"
              className="px-4 py-2 bg-slate-100 dark:bg-slate-900 hover:bg-slate-200 dark:hover:bg-slate-800 rounded-lg transition-colors"
            >
              ← Back to Tools
            </Link>
            <a
              href="#top"
              className="px-4 py-2 bg-slate-100 dark:bg-slate-900 hover:bg-slate-200 dark:hover:bg-slate-800 rounded-lg transition-colors"
            >
              ↑ Back to Top
            </a>
          </div>
        </div>

        {/* Sidebar - Table of Contents */}
        {sections.length > 0 && (
          <aside className="hidden lg:block">
            <nav className="sticky top-24 bg-slate-50 dark:bg-slate-900 p-6 rounded-lg border border-slate-200 dark:border-slate-800">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-white mb-4 uppercase tracking-wide">
                Contents
              </h3>
              <ul className="space-y-2 text-sm">
                {sections.map((section) => (
                  <li key={section.id} style={{ marginLeft: `${(section.level - 2) * 1}rem` }}>
                    <a
                      href={`#${section.id}`}
                      className="text-blue-600 dark:text-blue-400 hover:underline"
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
    </div>
  )
}

/**
 * Wrapper component to render markdown HTML with proper Tailwind styling
 */
function MarkdownContent({ htmlContent }: { htmlContent: string }) {
  return (
    <div
      className="markdown-content"
      dangerouslySetInnerHTML={{
        __html: sanitizeAndStyleHtml(htmlContent),
      }}
    />
  )
}

/**
 * Sanitize HTML to prevent XSS attacks, then apply Tailwind classes to markdown HTML elements
 * Uses DOMPurify for real XSS protection (Ma Protocol: NO FAKE SECURITY)
 */
function sanitizeAndStyleHtml(html: string): string {
  // STEP 1: REAL SANITIZATION with DOMPurify to prevent XSS attacks
  // This removes dangerous elements like <script>, event handlers, javascript: URLs, etc.
  const cleanHtml = DOMPurify.sanitize(html, {
    ALLOWED_TAGS: [
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
      'p', 'br', 'hr',
      'ul', 'ol', 'li',
      'table', 'thead', 'tbody', 'tr', 'th', 'td',
      'code', 'pre',
      'blockquote',
      'strong', 'em', 'b', 'i',
      'a',
      'div', 'span',
    ],
    ALLOWED_ATTR: ['href', 'class', 'id'],
    ALLOW_DATA_ATTR: false,
  });

  // STEP 2: Apply Tailwind classes to the now-safe HTML
  html = cleanHtml;
  // Add classes to H2 headings
  html = html.replace(/<h2([^>]*)>([^<]+)<\/h2>/g, (match, attrs, text) => {
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
    return `<h2 class="text-2xl font-bold mt-8 mb-4 text-slate-900 dark:text-white scroll-mt-24" id="${id}">${text}</h2>`
  })

  // Add classes to H3 headings
  html = html.replace(/<h3([^>]*)>([^<]+)<\/h3>/g, (match, attrs, text) => {
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
    return `<h3 class="text-xl font-semibold mt-6 mb-3 text-slate-800 dark:text-slate-100 scroll-mt-24" id="${id}">${text}</h3>`
  })

  // Add classes to paragraphs
  html = html.replace(/<p>([^<]+)<\/p>/g, '<p class="my-4 leading-relaxed text-slate-700 dark:text-slate-300">$1</p>')

  // Add classes to lists
  html = html.replace(
    /<ul>/g,
    '<ul class="list-disc list-inside my-4 ml-4 text-slate-700 dark:text-slate-300">'
  )
  html = html.replace(/<ol>/g, '<ol class="list-decimal list-inside my-4 ml-4 text-slate-700 dark:text-slate-300">')
  html = html.replace(/<li>/g, '<li class="my-2">')

  // Add classes to tables
  html = html.replace(
    /<table>/g,
    '<table class="min-w-full my-6 border-collapse border border-slate-300 dark:border-slate-700">'
  )
  html = html.replace(
    /<thead>/g,
    '<thead class="bg-slate-100 dark:bg-slate-800">'
  )
  html = html.replace(/<th>/g, '<th class="border border-slate-300 dark:border-slate-700 px-4 py-2 text-left font-semibold">')
  html = html.replace(/<td>/g, '<td class="border border-slate-300 dark:border-slate-700 px-4 py-2">')

  // Add classes to inline code
  html = html.replace(
    /<code(?!>)([^>]*)>/g,
    '<code class="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded font-mono text-sm">'
  )

  // Add classes to blockquotes
  html = html.replace(
    /<blockquote>/g,
    '<blockquote class="border-l-4 border-blue-500 pl-4 italic my-4 text-slate-700 dark:text-slate-300">'
  )

  // Add classes to strong/bold
  html = html.replace(/<strong>/g, '<strong class="font-semibold text-slate-900 dark:text-white">')

  // Add classes to emphasis
  html = html.replace(/<em>/g, '<em class="italic text-slate-700 dark:text-slate-300">')

  // Add classes to links
  html = html.replace(
    /<a(?!\s+class)([^>]*)href="([^"]+)"([^>]*)>([^<]+)<\/a>/g,
    '<a$1href="$2"$3 class="text-blue-600 dark:text-blue-400 hover:underline">$4</a>'
  )

  return html
}
