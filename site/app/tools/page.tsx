// T-SITE-001.9

import fs from 'node:fs'
import path from 'node:path'
import Link from 'next/link'
import type { Metadata } from 'next'
import { filenameToSlug } from '@/lib/tools'

export const metadata: Metadata = { title: 'Tools' }

type ToolCatalogEntry = {
  slug: string
  title: string
  description: string
}

function getToolsUsageDirectory(): string {
  // NOTE: site/lib/tools.ts currently points at ../docs/usage/tools, but the canonical
  // source of truth is ../src/assets/usage/tools. Some environments may provide a
  // symlink for backwards compatibility.
  const assetsDir = path.join(process.cwd(), '../src/assets/usage/tools')
  const docsDir = path.join(process.cwd(), '../docs/usage/tools')

  const hasMdFiles = (dir: string) => {
    if (!fs.existsSync(dir)) {
      return false
    }

    return fs.readdirSync(dir).some((f) => f.endsWith('.md'))
  }

  if (hasMdFiles(assetsDir)) {
    return assetsDir
  }

  if (hasMdFiles(docsDir)) {
    return docsDir
  }

  // Prefer the canonical location for clearer error messages.
  return assetsDir
}

function extractTitle(markdown: string, fallback: string): string {
  const match = markdown.match(/^#\s+(.+)$/m)
  return (match?.[1] ?? fallback).trim()
}

function stripMarkdownInline(text: string): string {
  return (
    text
      // Remove WikiLinks while keeping visible label-ish content minimal.
      .replace(/\[\[([^\]]+)\]\]/g, '$1')
      // [label](url) -> label
      .replace(/\[([^\]]+)\]\([^\)]+\)/g, '$1')
      // Inline code
      .replace(/`([^`]+)`/g, '$1')
      // Bold/italic markers
      .replace(/\*\*([^*]+)\*\*/g, '$1')
      .replace(/\*([^*]+)\*/g, '$1')
      .replace(/_([^_]+)_/g, '$1')
      // Collapse whitespace
      .replace(/\s+/g, ' ')
      .trim()
  )
}

function extractFirstParagraph(markdown: string): string {
  // Find the first H1, then take the first non-empty paragraph block that follows.
  const lines = markdown.split(/\r?\n/)

  const h1Index = lines.findIndex((l) => /^#\s+/.test(l))
  const start = h1Index >= 0 ? h1Index + 1 : 0

  // Skip initial blank lines.
  let i = start
  while (i < lines.length && lines[i].trim() === '') {
    i++
  }

  const paragraphLines: string[] = []
  for (; i < lines.length; i++) {
    const line = lines[i]

    // Stop at next heading.
    if (/^#{2,}\s+/.test(line)) {
      break
    }

    // Stop at blank line after collecting some content.
    if (line.trim() === '') {
      if (paragraphLines.length > 0) {
        break
      }
      continue
    }

    // Skip code fences for description.
    if (line.trim().startsWith('```')) {
      if (paragraphLines.length > 0) {
        break
      }
      continue
    }

    paragraphLines.push(line.trim())
  }

  return stripMarkdownInline(paragraphLines.join(' '))
}

function readToolCatalog(): ToolCatalogEntry[] {
  const toolsDir = getToolsUsageDirectory()

  if (!fs.existsSync(toolsDir)) {
    throw new Error(`Tools directory not found: ${toolsDir}`)
  }

  const toolFiles = fs
    .readdirSync(toolsDir)
    .filter((f) => f.endsWith('.md'))
    .sort((a, b) => a.localeCompare(b))

  return toolFiles.map((filename) => {
    const filePath = path.join(toolsDir, filename)
    const markdown = fs.readFileSync(filePath, 'utf-8')

    const fallbackTitle = filename.replace(/\.md$/i, '')
    const title = extractTitle(markdown, fallbackTitle)
    const description = extractFirstParagraph(markdown)

    return {
      slug: filenameToSlug(filename),
      title,
      description,
    }
  })
}

export default async function ToolsPage() {
  const tools = readToolCatalog()

  return (
    <div className="mx-auto px-4 py-20" style={{ maxWidth: '900px' }}>
      <style>{`
        .toolGrid {
          display: grid;
          grid-template-columns: 1fr;
          gap: 14px;
          margin-top: 28px;
        }

        @media (min-width: 720px) {
          .toolGrid {
            grid-template-columns: 1fr 1fr;
          }
        }

        .toolCard {
          border: 1px solid var(--color-border);
          background: var(--color-bg-surface);
          border-radius: 14px;
          padding: 16px 18px;
          transition: border-color 120ms ease, background-color 120ms ease;
        }

        .toolCard:hover {
          border-color: var(--color-accent);
          background: var(--color-bg-hover);
        }

        .toolTitle {
          font-size: 16px;
          font-weight: 600;
          line-height: 1.3;
          color: var(--color-text);
          margin: 0;
        }

        .toolDesc {
          margin-top: 8px;
          color: var(--color-text-secondary);
          line-height: 1.65;
          font-size: 14px;
        }
      `}</style>

      <h1 style={{ fontSize: '42px', lineHeight: 1.1, margin: 0 }}>
        Tools ({tools.length})
      </h1>
      <p style={{ marginTop: 14, color: 'var(--color-text-secondary)', lineHeight: 1.75 }}>
        A build-time catalog of Maenifold MCP tools.
      </p>

      <div className="toolGrid">
        {tools.map((tool) => (
          <div key={tool.slug} className="toolCard">
            <h2 className="toolTitle">
              <Link href={`/tools/${tool.slug}`}>{tool.title}</Link>
            </h2>
            {tool.description.length > 0 && <div className="toolDesc">{tool.description}</div>}
          </div>
        ))}
      </div>
    </div>
  )
}
