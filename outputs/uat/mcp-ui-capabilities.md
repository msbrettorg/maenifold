# MCP UI/UX Rendering Capabilities Test Plan

**Date**: 2026-07-11  
**Purpose**: Enumerate MCP rendering capabilities the maenifold desktop app must surface for parity with existing MCP clients (Claude Desktop, Zed, Cursor, VS Code Agent)

---

## Official Sources Consulted

| Source | URL |
|--------|-----|
| MCP Specification (2025-06-18) | https://modelcontextprotocol.io/specification/2025-06-18 |
| MCP Resources | https://modelcontextprotocol.io/docs/concepts/resources |
| MCP Prompts | https://modelcontextprotocol.io/docs/concepts/prompts |
| MCP Sampling | https://modelcontextprotocol.io/docs/concepts/sampling |
| MCP Roots | https://modelcontextprotocol.io/docs/concepts/roots |
| MCP Elicitation | https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation |
| MCP Tools | https://modelcontextprotocol.io/specification/2025-06-18/server/tools |
| MCP Completion | https://modelcontextprotocol.io/specification/2025-06-18/server/utilities/completion |
| MCP Inspector | https://github.com/modelcontextprotocol/inspector |
| TypeScript Schema | https://github.com/modelcontextprotocol/specification/blob/main/schema/2025-06-18/schema.ts |

---

## A. Resource Rendering

### MCP Resource Content Types (from spec)

The MCP spec defines resources as having two content formats:
- **TextResourceContents**: `{ uri, mimeType?, text }` — text-based content
- **BlobResourceContents**: `{ uri, mimeType?, blob }` — base64-encoded binary

**Source**: schema.ts `TextResourceContents`, `BlobResourceContents`

### Test Matrix

| MIME Type | Expected Rendering | Priority | Test Cases |
|-----------|-------------------|----------|------------|
| `text/markdown` | Rich markdown with syntax highlighting, links, headers | **P0** | 1. Render `# Heading` as H1<br>2. Render `**bold**` as bold<br>3. Render code fences with language highlighting<br>4. Render tables<br>5. Handle nested lists |
| `application/json` | Pretty-printed JSON with syntax highlighting | **P0** | 1. Collapse/expand nested objects<br>2. Syntax color keys vs values<br>3. Handle large JSON (>1MB) gracefully<br>4. Copy button |
| `text/plain` | Monospace text, line numbers | **P1** | 1. Preserve whitespace<br>2. Line wrap toggle<br>3. Line numbers optional |
| `text/x-rust` | Syntax highlighted Rust code | **P0** | 1. Proper keyword highlighting<br>2. String/comment coloring<br>3. Line numbers |
| `text/x-python` | Syntax highlighted Python code | **P0** | 1. f-string highlighting<br>2. Docstring rendering<br>3. Decorator syntax |
| `text/x-typescript`, `text/x-javascript` | Syntax highlighted TS/JS | **P0** | Same as above |
| `image/png`, `image/jpeg`, `image/gif`, `image/webp` | Inline image display | **P0** | 1. Scale to container<br>2. Zoom on click<br>3. Full resolution view<br>4. Save button |
| `image/svg+xml` | SVG render (not as code) | **P1** | 1. Scale properly<br>2. Preserve interactivity<br>3. Sanitize for XSS |
| `audio/wav`, `audio/mpeg`, `audio/ogg` | Audio player widget | **P1** | 1. Play/pause controls<br>2. Seek bar<br>3. Volume control<br>4. Duration display |
| `video/mp4`, `video/webm` | Video player widget | **P2** | 1. Play/pause/fullscreen<br>2. Seek bar<br>3. Volume control |
| `application/pdf` | PDF preview or download | **P2** | 1. Multi-page preview<br>2. Fallback to download |
| `application/octet-stream` (unknown) | Download button with filename | **P1** | 1. Show file size<br>2. Download action |

### Resource URI Handling

| Feature | Test Case | Priority |
|---------|-----------|----------|
| URI hover preview | Hover over `file:///path` shows preview tooltip | **P1** |
| Click-to-open | Click URI opens resource read dialog or viewer | **P1** |
| Resource subscriptions | Subscribe to resource, update UI when `notifications/resources/updated` received | **P1** |
| Resource list change | Handle `notifications/resources/list_changed` — refresh list | **P1** |

---

## B. Embedded Resources in Agent Messages

### MCP Content Block Types (from schema.ts)

```typescript
export type ContentBlock =
  | TextContent        // { type: "text", text, annotations? }
  | ImageContent       // { type: "image", data (base64), mimeType, annotations? }
  | AudioContent       // { type: "audio", data (base64), mimeType, annotations? }
  | ResourceLink       // { type: "resource_link", uri, name, mimeType?, annotations? }
  | EmbeddedResource;  // { type: "resource", resource: TextResourceContents|BlobResourceContents, annotations? }
```

**Source**: https://modelcontextprotocol.io/specification/2025-06-18/server/tools (Tool Result section)

### Test Matrix

| Content Block Type | Test Cases | Priority |
|-------------------|------------|----------|
| `TextContent` | 1. Render plain text<br>2. Respect `annotations.audience` (user vs assistant)<br>3. Apply `annotations.priority` for visual weight | **P0** |
| `ImageContent` | 1. Decode base64 + render inline<br>2. Support `image/png`, `image/jpeg`, `image/gif`, `image/svg+xml`<br>3. Click to enlarge | **P0** |
| `AudioContent` | 1. Decode base64 + create audio player<br>2. Support `audio/wav`, `audio/mpeg`, `audio/ogg` | **P1** |
| `ResourceLink` | 1. Render as clickable link with name/description<br>2. On click, fetch via `resources/read`<br>3. Show preview tooltip on hover | **P0** |
| `EmbeddedResource` | 1. Render inline based on mimeType<br>2. For text: display with appropriate syntax highlighting<br>3. For blob: render image/audio or offer download | **P0** |

### Annotations Handling

| Annotation | Behavior | Priority |
|------------|----------|----------|
| `audience: ["user"]` | Show to user, may hide from assistant context | **P1** |
| `audience: ["assistant"]` | Include in context, may de-emphasize in UI | **P1** |
| `priority: 0.0-1.0` | Higher priority = more prominent display | **P2** |
| `lastModified` | Show freshness indicator, sort by recency | **P2** |

### MCP Apps / ext-apps

**Note**: After extensive search, the MCP specification does not define "ext-apps" or embedded interactive widgets as a first-party feature. The spec focuses on static content types (text, image, audio, embedded resources). If maenifold wants rich interactive widgets, this would be an extension beyond current MCP.

**Recommendation**: Do not build ext-apps support unless/until MCP spec formalizes it.

---

## C. Markdown Rendering

### CommonMark Baseline (Required)

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Headings (`#` - `######`) | All 6 levels render with decreasing size | **P0** |
| Paragraphs | Separated by blank lines | **P0** |
| Bold (`**text**`) | Renders as bold | **P0** |
| Italic (`*text*`) | Renders as italic | **P0** |
| Code (`` `inline` ``) | Renders monospace inline | **P0** |
| Code fences (```) | Multi-line code block with language hint | **P0** |
| Links (`[text](url)`) | Clickable, opens external URLs | **P0** |
| Images (`![alt](url)`) | Inline image display | **P0** |
| Blockquotes (`>`) | Indented with visual indicator | **P0** |
| Lists (ordered/unordered) | Proper nesting and numbering | **P0** |
| Horizontal rules (`---`) | Renders as divider | **P1** |

### GFM Extensions (GitHub Flavored Markdown)

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Tables | Column alignment, header separation | **P0** |
| Strikethrough (`~~text~~`) | Line through text | **P1** |
| Task lists (`- [ ]`, `- [x]`) | Checkbox rendering (read-only OK) | **P1** |
| Autolinks (`https://...`) | Auto-linkify bare URLs | **P1** |

### Advanced Markdown

| Feature | Test Cases | Priority | Parity Target |
|---------|------------|----------|---------------|
| Footnotes (`[^1]`) | Superscript link, footnote at bottom | **P2** | — |
| Math: `$...$` inline | KaTeX/MathJax inline render | **P0** | Cursor ✓, Claude ✗ |
| Math: `$$...$$` block | Display math block | **P0** | Cursor ✓, Claude ✗ |
| Mermaid diagrams | Render in `\`\`\`mermaid` blocks | **P0** | Cursor ✓, VS Code partial |
| Syntax highlighting | Shiki/Prism/Highlight.js for code fences | **P0** | All ✓ |
| HTML passthrough | `<kbd>`, `<details>`, `<summary>`, `<sup>`, `<sub>` | **P1** | With XSS sanitization |
| WikiLinks (`[[...]]`) | Custom render as internal links to maenifold graph | **P0** | maenifold-specific |
| Heading anchors | Auto-generate `id` for headings | **P2** | — |
| TOC generation | Auto-generate table of contents | **P2** | — |

### Mermaid Diagram Types

| Diagram Type | Test Cases | Priority |
|--------------|------------|----------|
| Flowchart (`graph TD/LR`) | Nodes, edges, styling | **P0** |
| Sequence diagram | Actors, messages, loops | **P0** |
| Class diagram | Classes, inheritance, methods | **P1** |
| State diagram | States, transitions | **P1** |
| Entity-relationship | Entities, relationships | **P2** |
| Gantt chart | Tasks, timeline | **P2** |
| Git graph | Commits, branches | **P2** |

---

## D. Data Visualization

### Declarative Visualization in Code Blocks

| Format | Test Cases | Priority | Implementation |
|--------|------------|----------|----------------|
| Vega-Lite (`\`\`\`vega-lite`) | Parse JSON spec, render interactive chart | **P2** | vega-embed |
| Chart.js (`\`\`\`chartjs`) | Parse JSON config, render canvas chart | **P2** | chart.js |
| Plotly (`\`\`\`plotly`) | Parse JSON trace data, render interactive plot | **P2** | plotly.js |

**Note**: These are beyond standard MCP. Implement if maenifold wants advanced data visualization in chat.

### Graph Visualization (maenifold-specific)

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Cytoscape.js graph rendering | Render `maenifold.graph.Visualize` output | **P0** |
| Node click → navigate to concept | Click node opens concept detail | **P0** |
| Force-directed layout | Auto-layout for concept neighborhoods | **P1** |
| Zoom/pan controls | Mouse/touch interaction | **P1** |

---

## E. Multimedia Attachments — INPUT Side

### User Input Capabilities

| Feature | Test Cases | Priority | MCP Support |
|---------|------------|----------|-------------|
| File picker (any file) | 1. Open system file dialog<br>2. Select multiple files<br>3. Read file content | **P0** | Via prompts or custom |
| Drag-drop file | 1. Drag file onto chat<br>2. Show drop zone indicator<br>3. Process dropped files | **P0** | Client-side |
| Image paste from clipboard | 1. Ctrl/Cmd+V image<br>2. Convert to base64<br>3. Show preview before send | **P0** | Client-side |
| Audio recording (MediaRecorder) | 1. Start/stop recording<br>2. Show waveform/timer<br>3. Convert to base64 | **P1** | Client-side |
| Camera capture | 1. Access webcam<br>2. Take photo<br>3. Convert to base64 | **P2** | Client-side |
| File size limits | 1. Warn >10MB<br>2. Reject >50MB<br>3. Configurable limit | **P1** | Client policy |
| File type filters | 1. Filter by MIME type<br>2. Allow/deny list configuration | **P1** | Client policy |

---

## F. Multimedia Rendering — OUTPUT Side

### Display Capabilities

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Image full resolution | 1. Click to view original size<br>2. Zoom controls<br>3. Pan on large images | **P0** |
| Image save/download | 1. Right-click → Save<br>2. Download button | **P1** |
| Audio player | 1. Play/pause<br>2. Seek bar<br>3. Time display<br>4. Volume control | **P1** |
| Video player | 1. Play/pause/fullscreen<br>2. Seek bar<br>3. Volume control<br>4. Playback speed | **P2** |
| File download | 1. Show filename + size<br>2. Click to download<br>3. Progress indicator | **P0** |
| ZIP preview | 1. List archive contents<br>2. Extract individual files | **P3** |

---

## G. Slash Commands UI

### MCP Prompts as Slash Commands

**Source**: https://modelcontextprotocol.io/docs/concepts/prompts

MCP prompts are designed to be exposed as slash commands (see spec example image).

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Tab completion popup | 1. Type `/` → show available prompts<br>2. Fuzzy filter as user types<br>3. Keyboard navigation (↑↓ Enter) | **P0** |
| Argument schema display | 1. Show required vs optional args<br>2. Display argument descriptions<br>3. Type hints (string, enum, etc.) | **P0** |
| Argument completion | 1. Use `completion/complete` for argument values<br>2. Show dropdown of suggestions<br>3. Handle `hasMore` pagination | **P1** |
| History recall | 1. ↑ key recalls previous commands<br>2. History persists across sessions | **P2** |
| Prompt title display | 1. Show `title` if present, else `name`<br>2. Show `description` in tooltip | **P1** |

### Completion API Integration

**Source**: https://modelcontextprotocol.io/specification/2025-06-18/server/utilities/completion

| Feature | Test Cases | Priority |
|---------|------------|----------|
| `ref/prompt` completion | Autocomplete prompt arguments | **P1** |
| `ref/resource` completion | Autocomplete resource template URIs | **P1** |
| Context-aware completion | Pass previously resolved args in `context.arguments` | **P1** |
| Debounce requests | Rate-limit completion calls | **P1** |
| Cache results | Cache completions for repeated queries | **P2** |

---

## H. Sampling Support

### Server-Initiated LLM Requests

**Source**: https://modelcontextprotocol.io/docs/concepts/sampling

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Receive `sampling/createMessage` | Parse request, extract messages and preferences | **P0** |
| User approval UI | 1. Modal showing request details<br>2. Show prompt being sent<br>3. Approve/Deny buttons<br>4. Option to modify prompt before sending | **P0** |
| Model selection | 1. Honor `modelPreferences.hints`<br>2. Balance cost/speed/intelligence priorities<br>3. Allow user override | **P1** |
| Response to server | 1. Return `CreateMessageResult` with model, stopReason<br>2. Handle errors gracefully | **P0** |
| Rate limiting | 1. Limit sampling requests per server<br>2. Show warning if threshold exceeded | **P1** |

### Message Content Types in Sampling

| Type | Test Cases | Priority |
|------|------------|----------|
| TextContent | Render text in approval UI | **P0** |
| ImageContent | Render image preview in approval UI | **P0** |
| AudioContent | Render audio player in approval UI | **P1** |

---

## I. Elicitation (MCP 2025-06-18)

### Schema-Driven User Input

**Source**: https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Receive `elicitation/create` | Parse request, extract message and schema | **P0** |
| Form generation | Auto-generate form from `requestedSchema` | **P0** |
| Show message prompt | Display `message` field as form header | **P0** |

### Schema Type Rendering

| Schema Type | Form Control | Test Cases | Priority |
|-------------|--------------|------------|----------|
| `StringSchema` | Text input | 1. Validate `minLength`/`maxLength`<br>2. Apply `format` validation (email, uri, date, date-time) | **P0** |
| `NumberSchema` | Number input | 1. Validate `minimum`/`maximum`<br>2. Support `integer` vs `number` | **P0** |
| `BooleanSchema` | Checkbox or toggle | 1. Honor `default` value | **P0** |
| `EnumSchema` | Dropdown/radio buttons | 1. Show `enumNames` as display text<br>2. Submit `enum` value | **P0** |

### Response Actions

| Action | Test Case | Priority |
|--------|-----------|----------|
| `accept` | User submits → return data matching schema | **P0** |
| `decline` | User clicks "Decline" → return `{action: "decline"}` | **P0** |
| `cancel` | User closes dialog/presses Escape → return `{action: "cancel"}` | **P0** |

### Security

| Feature | Test Cases | Priority |
|---------|------------|----------|
| Schema validation | Validate user input against schema before submit | **P0** |
| Show requesting server | Display which server is requesting info | **P0** |
| Decline any time | Always show cancel/decline option | **P0** |

---

## Reference App Parity Matrix

| Feature | Claude Desktop | Zed | Cursor | VS Code Agent | Maenifold Target |
|---------|---------------|-----|--------|---------------|------------------|
| **Markdown basic** | ✓ partial | ✓ | ✓ | ✓ | ✓ |
| **Markdown tables** | ✗ (text only) | ✓ | ✓ | ✓ | ✓ |
| **Math (LaTeX)** | ✗ | ? | ✓ | ✗ | ✓ |
| **Mermaid diagrams** | ✗ | ✓ | ✓ | partial | ✓ |
| **Code highlighting** | minimal | ✓ | ✓ | ✓ | ✓ |
| **Inline images** | upload only | ✓ | ✓ | ✗ | ✓ |
| **Audio playback** | ? | ✗ | ? | ✗ | ✓ |
| **Video playback** | ✗ | ✗ | ✗ | ✗ | P2 |
| **Slash commands** | ✗ | ✓ | ✓ | ✓ | ✓ |
| **Elicitation UI** | ? | ? | ? | ? | ✓ |
| **Sampling approval** | implicit | ? | implicit | implicit | ✓ explicit |
| **WikiLinks** | ✗ | ✗ | ✗ | ✗ | ✓ (unique) |
| **Graph visualization** | ✗ | ✗ | ✗ | ✗ | ✓ (unique) |

---

## Priority Tiers

### P0 — Launch Blockers

These MUST work for maenifold to be usable as an MCP client:

1. TextContent / text/plain / text/markdown rendering
2. Code syntax highlighting (Shiki or equivalent)
3. ImageContent / image/* rendering (base64 decode + display)
4. ResourceLink click → resource read
5. EmbeddedResource rendering
6. Slash command UI (prompt discovery + invocation)
7. Elicitation form generation
8. Sampling approval UI

### P1 — Expected Features

Users expect these from any modern AI chat interface:

1. GFM tables, task lists, strikethrough
2. Math rendering (KaTeX)
3. Mermaid diagram rendering
4. Audio player for audio/wav, audio/mpeg
5. File drag-drop and clipboard paste
6. Completion API integration
7. Resource subscription handling

### P2 — Nice to Have

Features that differentiate but aren't critical:

1. Video player
2. Vega-Lite / Plotly / Chart.js visualization
3. PDF preview
4. Footnotes
5. TOC generation
6. History recall for slash commands
7. Annotations.priority visual weight

### P3 — Future/Experimental

1. ZIP archive preview
2. MCP Apps / ext-apps (if spec adds)
3. Interactive widgets in messages

---

## Implementation Recommendations

### Markdown Stack

Use a unified markdown pipeline:

```
User/Assistant message
      ↓
  markdown-it (CommonMark + GFM)
      ↓
  + markdown-it-mathjax3 (math)
      ↓
  + mermaid.js (diagrams)
      ↓
  + Shiki (code highlighting)
      ↓
  + DOMPurify (sanitization)
      ↓
  Rendered HTML in chat
```

### Resource Rendering

```typescript
function renderResource(content: TextResourceContents | BlobResourceContents) {
  if ('text' in content) {
    return renderByMimeType(content.mimeType, content.text);
  } else {
    const binary = base64Decode(content.blob);
    if (isImage(content.mimeType)) return renderImage(binary);
    if (isAudio(content.mimeType)) return renderAudioPlayer(binary);
    return renderDownload(binary, content.uri);
  }
}
```

### Elicitation Component

```typescript
function ElicitationForm({ schema, message, onSubmit, onDecline, onCancel }) {
  const fields = Object.entries(schema.properties).map(([name, def]) => {
    switch (def.type) {
      case 'string': return def.enum 
        ? <Select options={def.enum} labels={def.enumNames} />
        : <TextInput format={def.format} />;
      case 'number': return <NumberInput min={def.minimum} max={def.maximum} />;
      case 'boolean': return <Checkbox default={def.default} />;
    }
  });
  return (
    <Dialog title={message}>
      {fields}
      <Button onClick={onSubmit}>Submit</Button>
      <Button onClick={onDecline}>Decline</Button>
      <Button onClick={onCancel}>Cancel</Button>
    </Dialog>
  );
}
```

---

## Test Infrastructure

### Automated Visual Tests

1. **Snapshot testing**: Render each content type, compare to golden images
2. **MIME type coverage**: Ensure all spec MIME types have handlers
3. **Accessibility**: ARIA labels, keyboard navigation, screen reader support

### Manual Test Scenarios

1. Connect to MCP Inspector server, exercise all tool/resource types
2. Connect to real MCP server (e.g., filesystem), verify resource browsing
3. Trigger sampling request, verify approval UI
4. Trigger elicitation request, verify form generation

### Performance Tests

1. Large markdown documents (>100KB)
2. Many embedded images (>50)
3. Complex Mermaid diagrams (>100 nodes)
4. Rapid resource updates (subscriptions)

---

## Appendix: MCP Schema Content Types Reference

From `schema/2025-06-18/schema.ts`:

```typescript
// Content blocks that can appear in tool results, prompts, and messages
export type ContentBlock =
  | TextContent        // { type: "text", text: string }
  | ImageContent       // { type: "image", data: base64, mimeType: string }
  | AudioContent       // { type: "audio", data: base64, mimeType: string }
  | ResourceLink       // { type: "resource_link", uri, name, mimeType?, description? }
  | EmbeddedResource;  // { type: "resource", resource: { uri, text|blob, mimeType } }

// Resource contents from resources/read
export interface TextResourceContents {
  uri: string;
  mimeType?: string;
  text: string;
}

export interface BlobResourceContents {
  uri: string;
  mimeType?: string;
  blob: string; // base64
}
```

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-07-11 | Initial research from MCP spec 2025-06-18 |
