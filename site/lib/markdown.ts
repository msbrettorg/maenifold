// T-SITE-001.2a: Shared markdown rendering pipeline (build-time)
// RTM: FR-15.9, FR-15.10, FR-15.11, FR-15.12, NFR-15.5, NFR-15.6

import { execFileSync } from 'child_process'
import { mkdtempSync, readFileSync, rmdirSync, unlinkSync, writeFileSync } from 'fs'
import os from 'os'
import path from 'path'
import { marked } from 'marked'
import { createHighlighter } from 'shiki'

export type Highlighter = Awaited<ReturnType<typeof createHighlighter>>

const MermaidPalette = {
  background: '#1E1B22',
  primary: '#6AABB3',
  text: '#E8E0DB',
  line: '#2A2630',
} as const

const ShikiPalette = {
  background: '#18151C',
  foreground: '#D4CEC8',
  keyword: '#6AABB3',
  string: '#D4A76A',
  comment: '#6A645F',
  property: '#C4878A',
  number: '#8AAA7A',
  function: '#B8A0C8',
  punctuation: '#7A746F',
} as const

const WarmRestraintTheme = {
  name: 'warm-restraint',
  type: 'dark',
  colors: {
    'editor.background': ShikiPalette.background,
    'editor.foreground': ShikiPalette.foreground,
  },
  tokenColors: [
    {
      scope: ['comment', 'punctuation.definition.comment'],
      settings: { foreground: ShikiPalette.comment },
    },
    {
      scope: ['string', 'punctuation.definition.string'],
      settings: { foreground: ShikiPalette.string },
    },
    {
      scope: ['keyword', 'storage.type', 'storage.modifier'],
      settings: { foreground: ShikiPalette.keyword },
    },
    {
      scope: ['constant.numeric', 'constant.language.boolean'],
      settings: { foreground: ShikiPalette.number },
    },
    {
      scope: ['entity.name.function', 'support.function'],
      settings: { foreground: ShikiPalette.function },
    },
    {
      scope: ['variable.other.property', 'support.type.property-name', 'meta.object-literal.key'],
      settings: { foreground: ShikiPalette.property },
    },
    {
      scope: ['punctuation', 'meta.brace', 'meta.delimiter'],
      settings: { foreground: ShikiPalette.punctuation },
    },
  ],
} as const

const SupportedLangs = [
  'json',
  'typescript',
  'javascript',
  'bash',
  'python',
  'yaml',
  'tsx',
  'jsx',
  'csharp',
  'kql',
  'markdown',
  'css',
  'html',
  'toml',
] as const

let _highlighterPromise: Promise<Highlighter> | null = null

function decodeHtmlEntities(value: string): string {
  // Marked escapes code blocks; decode the common entities we expect.
  return value
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&#x27;/g, "'")
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function normalizeFenceLang(lang: string): string {
  const trimmed = (lang ?? '').trim().toLowerCase()
  if (!trimmed) {
    return 'text'
  }

  const aliases: Record<string, string> = {
    ts: 'typescript',
    js: 'javascript',
    sh: 'bash',
    shell: 'bash',
    zsh: 'bash',
    yml: 'yaml',
    md: 'markdown',
    'c#': 'csharp',
    cs: 'csharp',
    kusto: 'kql',
  }

  return aliases[trimmed] ?? trimmed
}

function filenameToSlug(filename: string): string {
  const baseName = filename.replace(/\.md$/i, '')
  return baseName.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase()
}

function rewriteRelativeHref(href: string | null | undefined): string | null {
  if (!href) {
    return href ?? null
  }

  const trimmed = href.trim()
  if (!trimmed) {
    return href
  }

  if (
    trimmed.startsWith('#') ||
    trimmed.startsWith('/') ||
    trimmed.startsWith('http://') ||
    trimmed.startsWith('https://') ||
    trimmed.startsWith('mailto:')
  ) {
    return href
  }

  // Minimal, heuristic link rewriting to make common repo-relative links usable on the site.
  // NOTE: renderMarkdown only receives the markdown string (no base path), so we can only
  // rewrite known patterns safely.
  const normalized = trimmed.replace(/^\.\//, '').replace(/^\.\.\//, '')
  const lower = normalized.toLowerCase()

  if (lower.endsWith('docs/readme.md') || lower === 'docs/readme') {
    return '/docs'
  }

  if (
    lower.includes('integrations/claude-code/plugin-maenifold') ||
    lower.includes('integrations/claude-code/plugin-product-team')
  ) {
    return '/plugins'
  }

  if (lower === 'readme.md' || lower.endsWith('/readme.md')) {
    return '/'
  }

  const toolsMatch = normalized.match(/(?:^|\/)(?:src\/assets\/usage\/tools|docs\/usage\/tools)\/([^\/]+\.md)$/i)
  if (toolsMatch) {
    return `/tools/${filenameToSlug(toolsMatch[1])}`
  }

  return href
}

async function replaceAsync(
  input: string,
  regex: RegExp,
  replacer: (...args: any[]) => Promise<string>
): Promise<string> {
  const matches: Array<{ start: number; end: number; match: RegExpExecArray }> = []

  let m: RegExpExecArray | null
  while ((m = regex.exec(input)) !== null) {
    matches.push({ start: m.index, end: m.index + m[0].length, match: m })
  }

  if (matches.length === 0) {
    return input
  }

  let out = ''
  let lastIndex = 0
  for (const item of matches) {
    out += input.slice(lastIndex, item.start)
    out += await replacer(...item.match)
    lastIndex = item.end
  }
  out += input.slice(lastIndex)
  return out
}

export async function getShikiHighlighter(): Promise<Highlighter> {
  if (_highlighterPromise) {
    return _highlighterPromise
  }

  _highlighterPromise = createHighlighter({
    themes: [WarmRestraintTheme as any],
    langs: [...SupportedLangs],
  })

  return _highlighterPromise
}

export async function highlightCode(code: string, lang: string): Promise<string> {
  const normalizedLang = normalizeFenceLang(lang)
  const highlighter = await getShikiHighlighter()

  try {
    return highlighter.codeToHtml(code, {
      lang: normalizedLang as any,
      theme: WarmRestraintTheme.name as any,
    })
  } catch {
    return `<pre><code>${escapeHtml(code)}</code></pre>`
  }
}

export async function renderMermaidToSvg(diagram: string): Promise<string> {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'maenifold-mermaid-'))
  const inputPath = path.join(dir, 'diagram.mmd')
  const outputPath = path.join(dir, 'diagram.svg')
  const configPath = path.join(dir, 'mermaid-config.json')

  const mmdcPath = path.join(process.cwd(), 'node_modules', '.bin', 'mmdc')

  const config = {
    theme: 'base',
    themeVariables: {
      background: MermaidPalette.background,
      primaryColor: MermaidPalette.primary,
      primaryBorderColor: MermaidPalette.line,
      primaryTextColor: MermaidPalette.text,
      lineColor: MermaidPalette.line,
      textColor: MermaidPalette.text,
      fontFamily:
        'ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial, "Apple Color Emoji", "Segoe UI Emoji"',
    },
  }

  writeFileSync(inputPath, diagram, 'utf8')
  writeFileSync(configPath, JSON.stringify(config), 'utf8')

  try {
    execFileSync(
      mmdcPath,
      ['-i', inputPath, '-o', outputPath, '--configFile', configPath, '--quiet'],
      { stdio: 'pipe' }
    )

    let svg = readFileSync(outputPath, 'utf8')
    svg = svg.replace(/^\s*<\?xml[^>]*\?>\s*/i, '')
    return svg
  } finally {
    // Best-effort cleanup (files and directory).
    try {
      unlinkSync(inputPath)
    } catch {
      // ignore
    }
    try {
      unlinkSync(outputPath)
    } catch {
      // ignore
    }
    try {
      unlinkSync(configPath)
    } catch {
      // ignore
    }
    try {
      rmdirSync(dir)
    } catch {
      // ignore
    }
  }
}

export async function renderMarkdown(source: string): Promise<string> {
  const mermaidBlocks: string[] = []

  // a) Extract ```mermaid blocks
  const withoutMermaid = source.replace(/```mermaid\s*([\s\S]*?)```/g, (_full, diagram: string) => {
    const id = mermaidBlocks.length
    mermaidBlocks.push(diagram.trim())
    return `\n\n<div data-mermaid-id="${id}"></div>\n\n`
  })

  // b) Parse remaining markdown to HTML using marked
  const renderer = new marked.Renderer()
  const baseLinkRenderer = renderer.link
  renderer.link = (link: any) => {
    const href = typeof link?.href === 'string' ? link.href : null
    const rewritten = rewriteRelativeHref(href)
    return baseLinkRenderer.call(renderer, {
      ...link,
      href: rewritten ?? link?.href,
    })
  }

  let html = marked.parse(withoutMermaid, {
    gfm: true,
    renderer,
  }) as string

  // c) Render Mermaid blocks to inline SVG
  const renderedSvgs = await Promise.all(mermaidBlocks.map((d) => renderMermaidToSvg(d)))

  // d) Replace placeholders with SVGs
  for (let i = 0; i < renderedSvgs.length; i++) {
    const placeholder = `<div data-mermaid-id="${i}"></div>`
    html = html.replace(placeholder, renderedSvgs[i])
  }

  // e) Highlight remaining code blocks using Shiki
  html = await replaceAsync(
    html,
    /<pre><code(?: class="([^"]*)")?>([\s\S]*?)<\/code><\/pre>/g,
    async (_full, classAttr: string | undefined, codeHtml: string) => {
      const classValue = classAttr ?? ''
      const langMatch = classValue.match(/language-([^\s]+)/)
      const lang = langMatch ? langMatch[1] : 'text'
      const decoded = decodeHtmlEntities(codeHtml)
      return highlightCode(decoded, lang)
    }
  )

  return html
}
