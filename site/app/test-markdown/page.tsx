// T-SITE-001.2a: RTM FR-15.9, FR-15.10, FR-15.11, FR-15.12 — Markdown rendering test page
// Server component: all rendering (Shiki, mmdc, remark) happens at build time.

import { renderMarkdown } from "@/lib/markdown";

const testDoc = `
# Test Document

This is a paragraph with **bold** and *italic*.

## Code Example

\`\`\`bash
brew install msbrettorg/tap/maenifold
\`\`\`

\`\`\`json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"] }
  }
}
\`\`\`

## Mermaid Diagram

\`\`\`mermaid
graph TD
  A[WikiLinks] --> B[Graph]
  B --> C[Search]
\`\`\`

| Column A | Column B |
|----------|----------|
| Cell 1   | Cell 2   |
`;

export default async function TestMarkdown() {
  const html = await renderMarkdown(testDoc);
  return (
    <main className="prose-width" style={{ padding: "5rem 1rem" }}>
      <div
        className="markdown-content"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </main>
  );
}
