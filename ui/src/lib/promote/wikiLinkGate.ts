// WikiLink pre-gate — a pure, framework-free check that mirrors the backend's
// validation closely enough to block a doomed write/edit BEFORE it round-trips
// to the daemon.
//
// The backend (crates/maenifold-core/src/memory/editing.rs:122) rejects when:
//     contains_malformed_wikilinks(content) || extract_wikilinks(content).is_empty()
// i.e. content is rejected if it has ANY malformed `[[...]]` syntax, OR if it
// has no valid `[[concept]]` at all. We reproduce BOTH conditions here.
//
// "Valid" (extract side) means:
//   - exactly double brackets — NOT triple/nested (`[[[x]]]`, `[[a[b]]`),
//   - the concept name contains no `[` or `]` (C# `[^\[\]]+` parity),
//   - the link is NOT inside a fenced or inline code span.
// "Malformed" (port of `contains_malformed_wikilinks`,
// editing_helpers.rs:99-130) is a RAW byte scan — it is NOT code-span aware, so
// a broken `[[` even inside code fences trips it, exactly like the backend.
//
// The backend collapses both failures into one message; this eager pre-gate
// surfaces a distinct malformed-syntax message so the user knows to fix the
// bracket syntax rather than add another link. The reject OUTCOME matches the
// backend in both cases.

export const WIKILINK_REQUIRED_MESSAGE =
  'Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.';

export const WIKILINK_MALFORMED_MESSAGE =
  'Content contains a malformed [[link]] — use simple double brackets like [[concept]].';

const OPEN_BRACKET = 0x5b; // '['
const CLOSE_BRACKET = 0x5d; // ']'

/**
 * Faithful port of the backend's `contains_malformed_wikilinks`
 * (crates/maenifold-core/src/memory/editing_helpers.rs:99-130). Scans the RAW
 * content (NOT code-span aware) for `[[` openers and flags as malformed when:
 *   - a nested `[`/`]` appears before the closing `]]`,
 *   - the close is a triple `]]]`, or
 *   - the `[[` is never closed.
 * Bracket chars are ASCII so a UTF-16 char scan is equivalent to the byte scan.
 */
export function containsMalformedWikiLinks(content: string): boolean {
  const len = content.length;
  let index = 0;

  while (index + 1 < len) {
    if (content.charCodeAt(index) !== OPEN_BRACKET || content.charCodeAt(index + 1) !== OPEN_BRACKET) {
      index += 1;
      continue;
    }

    let cursor = index + 2;
    while (cursor + 1 < len) {
      const current = content.charCodeAt(cursor);
      const next = content.charCodeAt(cursor + 1);
      if (current === CLOSE_BRACKET && next === CLOSE_BRACKET) {
        if (cursor + 2 < len && content.charCodeAt(cursor + 2) === CLOSE_BRACKET) {
          return true;
        }
        index = cursor + 2;
        break;
      }
      if (current === OPEN_BRACKET || current === CLOSE_BRACKET) {
        return true;
      }
      cursor += 1;
    }

    // Reached only when the inner loop fell through without finding `]]`
    // (after a match we break with cursor + 1 < len), i.e. an unclosed `[[`.
    if (cursor + 1 >= len) {
      return true;
    }
  }

  return false;
}

/**
 * Strip fenced (``` / ~~~) and inline (`...`) code spans so links inside code
 * are ignored, matching the backend which skips WikiLinks inside code blocks.
 */
function stripCodeSpans(content: string): string {
  // Remove fenced code blocks first (``` or ~~~ delimited), then inline spans.
  const withoutFenced = content.replace(/(^|\n)[ \t]*(```|~~~)[^\n]*\n[\s\S]*?(\n[ \t]*\2[^\n]*(?=\n|$)|$)/g, '\n');
  return withoutFenced.replace(/`[^`\n]*`/g, ' ');
}

/**
 * Extract the valid `[[concept]]` links from content (code spans removed). A
 * candidate is rejected when the opening `[[` is preceded by another `[` (triple
 * bracket), when the closing `]]` is followed by another `]`, or when the inner
 * concept text contains a `[` or `]` (nested brackets).
 */
export function extractValidWikiLinks(content: string): string[] {
  const text = stripCodeSpans(content);
  const links: string[] = [];
  let index = text.indexOf('[[');

  while (index !== -1) {
    // Reject triple-open: a `[` immediately before the `[[`.
    const tripleOpen = index > 0 && text[index - 1] === '[';
    const close = text.indexOf(']]', index + 2);
    if (close === -1) break;

    const inner = text.slice(index + 2, close);
    const hasNestedBracket = inner.includes('[') || inner.includes(']');
    // Reject triple-close: a `]` immediately after the `]]`.
    const tripleClose = text[close + 2] === ']';

    if (!tripleOpen && !tripleClose && !hasNestedBracket && inner.trim().length > 0) {
      links.push(inner.trim());
    }

    index = text.indexOf('[[', close + 2);
  }

  return links;
}

/** True when content carries at least one valid `[[concept]]` link. */
export function hasValidWikiLink(content: string): boolean {
  return extractValidWikiLinks(content).length > 0;
}

export interface WikiLinkGateResult {
  ok: boolean;
  /** Null when ok; otherwise the backend-parity rejection message. */
  message: string | null;
  links: string[];
}

/**
 * The mandatory pre-gate. Mirrors the backend's
 * `contains_malformed_wikilinks(c) || extract_wikilinks(c).is_empty()` reject:
 * malformed `[[...]]` syntax fails first (distinct message), then content with
 * no valid `[[concept]]` fails with the required-link message.
 */
export function wikiLinkGate(content: string): WikiLinkGateResult {
  if (containsMalformedWikiLinks(content)) {
    return { ok: false, message: WIKILINK_MALFORMED_MESSAGE, links: [] };
  }
  const links = extractValidWikiLinks(content);
  if (links.length === 0) {
    return { ok: false, message: WIKILINK_REQUIRED_MESSAGE, links: [] };
  }
  return { ok: true, message: null, links };
}
