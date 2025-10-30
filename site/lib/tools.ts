import path from 'path'
import fs from 'fs'
import { marked } from 'marked'
import { createHighlighter } from 'shiki'

// Cache for highlighter instance (expensive to create)
let highlighter: Awaited<ReturnType<typeof createHighlighter>> | null = null

/**
 * Convert tool filename to URL slug
 * writememory.md -> write-memory
 * buildcontext.md -> build-context
 * repairconcepts.md -> repair-concepts
 */
export function filenameToSlug(filename: string): string {
  // Remove .md extension
  const baseName = filename.replace('.md', '')

  // Insert hyphens before capital letters, then convert to lowercase
  const withHyphens = baseName
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .toLowerCase()

  return withHyphens
}

/**
 * Convert slug back to filename
 * write-memory -> writememory.md
 */
export function slugToFilename(slug: string): string {
  // Remove hyphens and keep lowercase (files are all lowercase)
  return `${slug.replace(/-/g, '')}.md`
}

/**
 * Get all available tool slugs
 */
export function getAllToolSlugs(): string[] {
  const toolsDir = path.join(process.cwd(), '../src/assets/usage/tools')

  if (!fs.existsSync(toolsDir)) {
    console.warn(`Tools directory not found: ${toolsDir}`)
    return []
  }

  const files = fs.readdirSync(toolsDir).filter((file) => file.endsWith('.md'))

  return files.map(filenameToSlug)
}

/**
 * Load tool markdown content
 */
function loadToolMarkdown(slug: string): string {
  const filename = slugToFilename(slug)
  const toolPath = path.join(process.cwd(), '../src/assets/usage/tools', filename)

  if (!fs.existsSync(toolPath)) {
    throw new Error(`Tool not found: ${filename}`)
  }

  return fs.readFileSync(toolPath, 'utf-8')
}

/**
 * Initialize syntax highlighter
 */
async function getHighlighterInstance() {
  if (!highlighter) {
    highlighter = await createHighlighter({
      themes: ['github-light', 'github-dark'],
      langs: ['json', 'typescript', 'javascript', 'bash', 'python', 'yaml', 'tsx', 'jsx'],
    })
  }
  return highlighter
}

/**
 * Parse tool markdown and return structured data
 */
export async function loadToolData(slug: string) {
  const markdown = loadToolMarkdown(slug)
  // const highlighter = await getHighlighterInstance()

  // Parse markdown to HTML
  const htmlContent = await marked(markdown)

  // Extract title from first H1
  const titleMatch = markdown.match(/^# (.+)$/m)
  const title = titleMatch ? titleMatch[1] : 'Tool Documentation'

  // Extract description (first paragraph after title)
  const descMatch = markdown.match(/^# .+\n\n([\s\S]+?)(?:\n\n##|\n$)/)
  const description = descMatch ? descMatch[1].replace(/\[\[.+?\]\]/g, '$&').slice(0, 200) : ''

  return {
    slug,
    title,
    description,
    htmlContent,
    markdown,
  }
}

/**
 * Extract section headings from HTML
 */
export function extractSections(htmlContent: string): Array<{ id: string; title: string; level: number }> {
  const sections: Array<{ id: string; title: string; level: number }> = []
  const headingRegex = /<h([2-3])[^>]*[^>]*>(.+?)<\/h\1>/g
  const idCounts = new Map<string, number>() // Track duplicate IDs

  let match
  while ((match = headingRegex.exec(htmlContent)) !== null) {
    const level = parseInt(match[1])
    const title = match[2].replace(/<[^>]*>/g, '') // Remove any nested tags
    let id = title
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')

    // Handle duplicate IDs by appending a counter
    const count = idCounts.get(id) || 0
    if (count > 0) {
      id = `${id}-${count}`
    }
    idCounts.set(id, count + 1)

    sections.push({ id, title, level })
  }

  return sections
}
