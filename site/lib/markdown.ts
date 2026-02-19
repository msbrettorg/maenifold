// T-SITE-001.2a: RTM FR-15.10, FR-15.11, FR-15.12 — Unified markdown rendering pipeline
// Pipeline: extract Mermaid → extract code → parse markdown → render Mermaid → render code → return HTML

import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkRehype from "remark-rehype";
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import rehypeStringify from "rehype-stringify";

// Extend the default sanitization schema to allow elements used in tool docs
// (details/summary, class attributes for styling) while stripping <script>,
// <iframe>, event handlers, etc.
const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [
    ...(defaultSchema.tagNames ?? []),
    "details",
    "summary",
    "figure",
    "figcaption",
  ],
  attributes: {
    ...defaultSchema.attributes,
    // Allow class on any element (needed for code highlighting, mermaid, etc.)
    "*": [...(defaultSchema.attributes?.["*"] ?? []), "className", "class"],
  },
};

import { renderMermaid } from "./mermaid";
import { highlightCode } from "./shiki";

// Regex patterns for fenced code block extraction
// Matches ```lang\ncontent\n``` with optional trailing whitespace/newline
const MERMAID_BLOCK_RE = /```mermaid\n([\s\S]*?)```/g;
const CODE_BLOCK_RE = /```([^\n]*)\n([\s\S]*?)```/g;

interface ExtractedBlock {
  lang: string;
  code: string;
}

/**
 * Renders a markdown document to an HTML string at build time.
 *
 * Pipeline steps:
 * 1. Extract Mermaid fenced blocks and replace with placeholder tokens
 * 2. Extract all other fenced code blocks and replace with placeholder tokens
 * 3. Parse remaining markdown with unified (remark → rehype → HTML)
 * 4. Render each Mermaid block via mmdc and substitute placeholders
 * 5. Highlight each code block via Shiki and substitute placeholders
 *
 * @param source - The raw markdown string to render
 * @returns The final HTML string with inline SVG diagrams and highlighted code
 */
export async function renderMarkdown(source: string): Promise<string> {
  // --- Step 1: Extract Mermaid blocks ---
  const mermaidBlocks: string[] = [];
  const afterMermaid = source.replace(MERMAID_BLOCK_RE, (_match, code: string) => {
    const index = mermaidBlocks.length;
    mermaidBlocks.push(code);
    return `<!--MERMAID_${index}-->`;
  });

  // --- Step 2: Extract remaining code blocks ---
  const codeBlocks: ExtractedBlock[] = [];
  const afterCode = afterMermaid.replace(CODE_BLOCK_RE, (_match, lang: string, code: string) => {
    const index = codeBlocks.length;
    codeBlocks.push({ lang: (lang ?? "").trim(), code });
    return `<!--CODE_${index}-->`;
  });

  // --- Step 3: Parse markdown to HTML ---
  const file = await unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkRehype, { allowDangerousHtml: true })
    .use(rehypeRaw)
    .use(rehypeSanitize, sanitizeSchema)
    .use(rehypeStringify)
    .process(afterCode);

  let html = String(file);

  // --- Step 4: Render Mermaid placeholders ---
  for (let i = 0; i < mermaidBlocks.length; i++) {
    const svg = await renderMermaid(mermaidBlocks[i]);
    html = html.replace(
      `<!--MERMAID_${i}-->`,
      `<figure class="mermaid-diagram">${svg}</figure>`
    );
  }

  // --- Step 5: Render code block placeholders ---
  for (let i = 0; i < codeBlocks.length; i++) {
    const { lang, code } = codeBlocks[i];
    const highlighted = await highlightCode(code, lang);
    html = html.replace(
      `<!--CODE_${i}-->`,
      `<div class="code-block code-width">${highlighted}</div>`
    );
  }

  return html;
}
