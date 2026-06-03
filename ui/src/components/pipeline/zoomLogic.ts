// Pure, framework-free logic for the SemanticZoom overlay (T-600.6).
//
// Three responsibilities, each unit-testable without a DOM:
//   1. sniffContentFormat — MIME sniffing for a node's `output` string so the
//      ZoomOutputView picks JSON / Markdown / plain-text rendering by INSPECTING
//      the bytes, never by trusting a node field.
//   2. parseAssembledPrompt — best-effort split of an assembled prompt into
//      predecessor / ambient / instruction sections, keyed on recognizable
//      header lines, with an honest whole-text fallback when no headers parse.
//   3. availableSurfaces — the node → which-of-the-5-renderers selector the
//      overlay shell uses to decide which tabs to offer.
//
// No Vue, no marked: the overlay components consume these results.

import {
  isSequentialThinkingViewNode,
  isToolViewNode,
  isWorkflowViewNode,
  type ViewNode,
} from '../../lib/pipeline-types';

// ---------------------------------------------------------------------------
// 1. MIME sniffing
// ---------------------------------------------------------------------------

export type SniffedFormat = 'json' | 'markdown' | 'text';

export interface JsonParseResult {
  ok: boolean;
  value: unknown;
}

/**
 * Attempt to parse `content` as a JSON object or array. Scalars (bare numbers,
 * quoted strings, booleans, null) are intentionally NOT treated as structured
 * JSON — pretty-printing them adds nothing, so they fall through to the text /
 * markdown sniffers.
 */
export function tryParseJsonStructure(content: string): JsonParseResult {
  const trimmed = content.trim();
  if (!trimmed) return { ok: false, value: undefined };
  const first = trimmed[0];
  if (first !== '{' && first !== '[') return { ok: false, value: undefined };
  try {
    const value = JSON.parse(trimmed) as unknown;
    if (value !== null && typeof value === 'object') {
      return { ok: true, value };
    }
    return { ok: false, value: undefined };
  } catch {
    return { ok: false, value: undefined };
  }
}

/** Pretty-print a parsed JSON value with 2-space indentation. */
export function prettyJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

// Markdown markers tested line-by-line (multiline) or across the whole string.
// Any single hit classifies the content as Markdown.
const MARKDOWN_LINE_PATTERNS: RegExp[] = [
  /^#{1,6}\s+\S/m, // ATX heading
  /^\s{0,3}>[ \t]/m, // blockquote
  /^\s{0,3}[-*+][ \t]+\S/m, // unordered list
  /^\s{0,3}\d+\.[ \t]+\S/m, // ordered list
  /^\s*\|.+\|\s*$/m, // table row
  /^\s*(?:```|~~~)/m, // fenced code
];

const MARKDOWN_INLINE_PATTERNS: RegExp[] = [
  /\[[^\]]+\]\([^)]+\)/, // link
  /\*\*[^*\n]+\*\*/, // bold
  /(?:^|\s)`[^`\n]+`/, // inline code
];

/**
 * Sniff the rendering format for an output string. Order is deliberate:
 * structured JSON wins first (it's the most certain signal), then Markdown by
 * marker heuristics, otherwise plain text.
 */
export function sniffContentFormat(content: string): SniffedFormat {
  if (!content || !content.trim()) return 'text';
  if (tryParseJsonStructure(content).ok) return 'json';
  const isMarkdown =
    MARKDOWN_LINE_PATTERNS.some((re) => re.test(content)) ||
    MARKDOWN_INLINE_PATTERNS.some((re) => re.test(content));
  return isMarkdown ? 'markdown' : 'text';
}

// ---------------------------------------------------------------------------
// 2. Assembled-prompt section parse
// ---------------------------------------------------------------------------

export type PromptSectionKind = 'predecessor' | 'ambient' | 'instruction';

export interface PromptSections {
  // True when at least one recognizable header mapped content into one of the
  // three buckets below. False means the whole prompt is rendered as one block.
  parsed: boolean;
  predecessor?: string;
  ambient?: string;
  instruction?: string;
  // Always the full original text, so callers can offer a "raw" view.
  whole: string;
}

interface RawSection {
  header: string;
  body: string;
}

// A line is treated as a section header if it is a markdown heading, a bracket
// banner, a bare "Label:" line, or an =====/----- banner with a caption.
const HEADER_PATTERNS: RegExp[] = [
  /^#{1,6}\s+(.+?)\s*$/,
  /^\[(.+?)\]\s*$/,
  /^([A-Za-z][A-Za-z0-9 _\-/]{0,48}):\s*$/,
  /^[=-]{3,}\s*(.+?)\s*[=-]{0,}$/,
];

function matchHeader(line: string): string | null {
  for (const re of HEADER_PATTERNS) {
    const m = re.exec(line);
    if (m && m[1] && m[1].trim()) return m[1].trim();
  }
  return null;
}

const KIND_KEYWORDS: Record<PromptSectionKind, RegExp> = {
  predecessor: /\b(predecessor|upstream|prior|previous|inbound|incoming|input|from)\b/i,
  ambient: /\b(ambient|context|background|memory|environment|surrounding|knowledge)\b/i,
  instruction: /\b(instruction|task|directive|objective|goal|prompt|request|ask|step)\b/i,
};

// Checked in priority order so a header that matches several keyword families
// lands in the most specific bucket deterministically.
const KIND_ORDER: PromptSectionKind[] = ['predecessor', 'ambient', 'instruction'];

function classifyHeader(header: string): PromptSectionKind | null {
  for (const kind of KIND_ORDER) {
    if (KIND_KEYWORDS[kind].test(header)) return kind;
  }
  return null;
}

function splitRawSections(text: string): RawSection[] {
  const lines = text.split(/\r?\n/);
  const sections: RawSection[] = [];
  let current: RawSection | null = null;
  for (const line of lines) {
    const header = matchHeader(line);
    if (header !== null) {
      if (current) sections.push(current);
      current = { header, body: '' };
    } else if (current) {
      current.body += current.body ? `\n${line}` : line;
    }
  }
  if (current) sections.push(current);
  return sections;
}

/**
 * Parse an assembled prompt into predecessor / ambient / instruction parts.
 *
 * HEURISTIC: the live backend currently emits `assembledPrompt: null`, so the
 * exact delimiter format is unconfirmed. This parser keys on header-like lines
 * (markdown headings, `[Banner]`, `Label:`, `=== banner ===`) and classifies
 * each section by keyword. When no header maps into a bucket it returns
 * `parsed: false` and the caller renders `whole` as a single section.
 */
export function parseAssembledPrompt(text: string): PromptSections {
  const whole = text ?? '';
  const buckets: Partial<Record<PromptSectionKind, string>> = {};
  for (const section of splitRawSections(whole)) {
    const kind = classifyHeader(section.header);
    const body = section.body.trim();
    if (!kind || !body) continue;
    buckets[kind] = buckets[kind] ? `${buckets[kind]}\n\n${body}` : body;
  }
  const parsed = KIND_ORDER.some((kind) => buckets[kind] !== undefined);
  return {
    parsed,
    predecessor: buckets.predecessor,
    ambient: buckets.ambient,
    instruction: buckets.instruction,
    whole,
  };
}

// ---------------------------------------------------------------------------
// 3. Node → available zoom surfaces
// ---------------------------------------------------------------------------

// 'live' is the read-only live ACP session stream (T-600.7). It is NOT produced
// by `availableSurfaces` (which only reports node-content surfaces, keeping its
// pure-function tests stable); the overlay composes it in separately for any
// inspectable (non-tool) node.
export type ZoomSurface = 'live' | 'prompt' | 'thinking' | 'workflow' | 'output' | 'provenance';

function hasText(value: string | null | undefined): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

/**
 * Which of the five content renderers a node can feed. A node may expose
 * several (e.g. a sequentialThinking node carries assembledPrompt + thinkingTrace
 * + output). Order defines the tab order in the overlay shell.
 */
export function availableSurfaces(node: ViewNode): ZoomSurface[] {
  const surfaces: ZoomSurface[] = [];

  const assembledPrompt =
    isWorkflowViewNode(node) || isSequentialThinkingViewNode(node) || node.type === 'prompt'
      ? (node as { assembledPrompt?: string | null }).assembledPrompt
      : undefined;
  if (hasText(assembledPrompt)) surfaces.push('prompt');

  if (isSequentialThinkingViewNode(node) && (node.thinkingTrace?.length ?? 0) > 0) {
    surfaces.push('thinking');
  }

  if (isWorkflowViewNode(node) && (node.workflowSteps?.length ?? 0) > 0) {
    surfaces.push('workflow');
  }

  if (hasText(node.output)) surfaces.push('output');

  if (isToolViewNode(node) && node.toolProvenance != null) surfaces.push('provenance');

  return surfaces;
}

// ---------------------------------------------------------------------------
// 4. Node config preview entries (inspector "config" surface)
// ---------------------------------------------------------------------------

// `block: true` entries are object/schema fields rendered as pretty JSON in a
// <pre>; `block: false` entries are scalar/list fields rendered inline.
export interface ConfigEntry {
  label: string;
  value: string;
  block: boolean;
}

// The config-bearing optional fields that may appear on any ViewNode variant.
// ViewNode's runtime shapes do NOT carry joinPolicy/retry* (those live only on
// the definition-side PipelineNodeBase), so they are intentionally absent here.
interface ConfigBearing {
  agentConfig?: { [k: string]: unknown } | null;
  toolArgs?: Record<string, unknown> | null;
  inputSchema?: Record<string, unknown> | null;
  outputSchema?: Record<string, unknown> | null;
  allowedTools?: string[] | null;
  toolMode?: string | null;
  thoughtLevel?: string | null;
  analysisType?: string | null;
  totalThoughts?: number | null;
  maxTurns?: number | null;
  cwd?: string | null;
  timeout?: number | null;
}

function isNonEmptyRecord(value: unknown): value is Record<string, unknown> {
  return (
    value != null &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    Object.keys(value as Record<string, unknown>).length > 0
  );
}

function pushScalar(entries: ConfigEntry[], label: string, value: unknown): void {
  if (value === null || value === undefined) return;
  if (typeof value === 'string' && value.trim().length === 0) return;
  entries.push({ label, value: String(value), block: false });
}

function pushJson(entries: ConfigEntry[], label: string, value: unknown): void {
  if (!isNonEmptyRecord(value)) return;
  entries.push({ label, value: prettyJson(value), block: true });
}

/**
 * Build the inspector's config preview for a node. Reads every config-bearing
 * optional field that exists on the node's runtime shape, omitting null / empty
 * values (no blank rows). Object fields (agentConfig / toolArgs / schemas) are
 * pretty-printed as JSON; scalars and the allowedTools list render inline.
 */
export function nodeConfigEntries(node: ViewNode): ConfigEntry[] {
  const c = node as ConfigBearing;
  const entries: ConfigEntry[] = [];

  pushJson(entries, 'agentConfig', c.agentConfig);
  pushJson(entries, 'toolArgs', c.toolArgs);
  if (c.allowedTools && c.allowedTools.length > 0) {
    entries.push({ label: 'allowedTools', value: c.allowedTools.join(', '), block: false });
  }
  pushScalar(entries, 'toolMode', c.toolMode);
  pushScalar(entries, 'thoughtLevel', c.thoughtLevel);
  pushScalar(entries, 'analysisType', c.analysisType);
  pushScalar(entries, 'totalThoughts', c.totalThoughts);
  pushScalar(entries, 'maxTurns', c.maxTurns);
  pushScalar(entries, 'cwd', c.cwd);
  pushScalar(entries, 'timeout', c.timeout);
  pushJson(entries, 'inputSchema', c.inputSchema);
  pushJson(entries, 'outputSchema', c.outputSchema);

  return entries;
}
