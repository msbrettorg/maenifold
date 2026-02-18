// T-SITE-001.7

import fs from 'fs'
import path from 'path'
import { renderMarkdown } from '../../lib/markdown'
import { MarkdownContent } from '../components/MarkdownContent'

export const metadata = {
  title: 'Documentation',
}

export default async function DocsPage() {
  // Read at build time (server component)
  const docsPath = path.join(process.cwd(), '../docs/README.md')
  const source = fs.readFileSync(docsPath, 'utf-8')
  const html = await renderMarkdown(source)

  return (
    <div className="mx-auto px-4 py-20" style={{ maxWidth: '900px' }}>
      <article className="prose dark:prose-invert max-w-none">
        <MarkdownContent html={html} />
      </article>
    </div>
  )
}
