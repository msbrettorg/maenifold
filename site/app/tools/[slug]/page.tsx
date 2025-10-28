import { Metadata } from 'next'
import Link from 'next/link'
import { getAllToolSlugs, loadToolData, extractSections } from '@/lib/tools'

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
  params: { slug: string }
}): Promise<Metadata> {
  try {
    const toolData = await loadToolData(params.slug)
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
export default async function ToolPage({ params }: { params: { slug: string } }) {
  let toolData
  try {
    toolData = await loadToolData(params.slug)
  } catch (error) {
    return (
      <div className="min-h-screen bg-white dark:bg-slate-950">
        <div className="max-w-4xl mx-auto px-4 py-12">
          <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">Tool Not Found</h1>
          <p className="text-slate-600 dark:text-slate-300 mb-6">
            The tool "{params.slug}" could not be found.
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

      <div className="max-w-4xl mx-auto px-4 py-12 grid grid-cols-1 lg:grid-cols-4 gap-8">
        {/* Main content */}
        <div className="lg:col-span-3">
          {/* Header */}
          <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">
            {toolData.title}
          </h1>

          {/* Table of Contents */}
          {sections.length > 0 && (
            <nav className="mb-8 bg-slate-50 dark:bg-slate-900 p-4 rounded-lg border border-slate-200 dark:border-slate-800">
              <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-3 uppercase tracking-wide">
                Contents
              </h2>
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
          )}

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

        {/* Sidebar */}
        <aside className="hidden lg:block">
          <div className="sticky top-24 bg-slate-50 dark:bg-slate-900 p-6 rounded-lg border border-slate-200 dark:border-slate-800">
            <h3 className="font-semibold text-slate-900 dark:text-white mb-4">About This Tool</h3>
            <dl className="space-y-4 text-sm">
              <div>
                <dt className="font-medium text-slate-700 dark:text-slate-300">Tool Name</dt>
                <dd className="text-slate-600 dark:text-slate-400 mt-1">{toolData.title}</dd>
              </div>
              <div>
                <dt className="font-medium text-slate-700 dark:text-slate-300">Slug</dt>
                <dd className="text-slate-600 dark:text-slate-400 mt-1 font-mono text-xs break-all">
                  {params.slug}
                </dd>
              </div>
              <div>
                <dt className="font-medium text-slate-700 dark:text-slate-300">Sections</dt>
                <dd className="text-slate-600 dark:text-slate-400 mt-1">{sections.length} sections</dd>
              </div>
            </dl>
          </div>
        </aside>
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
        __html: sanitizeHtml(htmlContent),
      }}
    />
  )
}

/**
 * Apply Tailwind classes to markdown HTML elements
 */
function sanitizeHtml(html: string): string {
  // Add classes to H2 headings
  html = html.replace(/<h2([^>]*)>([^<]+)<\/h2>/g, () => {
    const text = arguments[2] as string
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
    return `<h2 class="text-2xl font-bold mt-8 mb-4 text-slate-900 dark:text-white scroll-mt-24" id="${id}">${text}</h2>`
  })

  // Add classes to H3 headings
  html = html.replace(/<h3([^>]*)>([^<]+)<\/h3>/g, () => {
    const text = arguments[2] as string
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
