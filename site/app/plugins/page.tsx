// T-SITE-001.8

import fs from 'fs'
import path from 'path'
import { renderMarkdown } from '../../lib/markdown'
import { MarkdownContent } from '../components/MarkdownContent'

export const metadata = {
  title: 'Plugins',
}

export default async function PluginsPage() {
  // Read at build time (server component)
  const maenifoldPluginPath = path.join(
    process.cwd(),
    '../integrations/claude-code/plugin-maenifold/README.md'
  )
  const productTeamPluginPath = path.join(
    process.cwd(),
    '../integrations/claude-code/plugin-product-team/README.md'
  )

  const maenifoldSource = fs.readFileSync(maenifoldPluginPath, 'utf-8')
  const productTeamSource = fs.readFileSync(productTeamPluginPath, 'utf-8')

  const maenifoldHtml = await renderMarkdown(maenifoldSource)
  const productTeamHtml = await renderMarkdown(productTeamSource)

  return (
    <div className="mx-auto px-4 py-20" style={{ maxWidth: '900px' }}>
      <article className="prose dark:prose-invert max-w-none">
        <MarkdownContent html={maenifoldHtml} />
      </article>

      <hr style={{ borderColor: 'var(--color-border)' }} />

      <article className="prose dark:prose-invert max-w-none">
        <MarkdownContent html={productTeamHtml} />
      </article>
    </div>
  )
}
