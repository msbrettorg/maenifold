// Surgery edge construction + preflight (T-600.5). Pure helpers that build the
// `PipelineEdge[]` graft/join must send to the daemon and the collision/identity
// preflights the backend would otherwise reject after a round-trip.
//
// `PipelineEdge.type` is REQUIRED (lib/pipeline-types.ts:69-75) — the gateway's
// oneOf rejects a bare {source,target}. Conditional edges additionally require
// both `path` and `equals` (pipelineDesignOps.ts:160-168, mirrored in the
// backend validator). New surgery edges therefore default to `success` and only
// become `conditional` once both conditional fields are supplied.
//
// Collisions: graft (handler_surgery_graft.rs:125-132) and join
// (handler_surgery_join.rs:54-64) both reject overlapping node IDs. We preflight
// the intersection so the user fixes it before any server mutation.

import type { EdgeType, PipelineEdge } from './pipeline-types';

// The edge types a surgery user may pick. `always` and `success` need no extra
// fields; `conditional` requires `path` + `equals`.
export const SURGERY_EDGE_TYPES: readonly EdgeType[] = ['success', 'always', 'conditional'];

export const DEFAULT_EDGE_TYPE: EdgeType = 'success';

// A user-facing, partially-filled edge before it is validated into a PipelineEdge.
export interface EdgeDraft {
  source: string;
  target: string;
  type: EdgeType;
  // Conditional-only. Edited as text; `equals` is sent as a string (the backend
  // condition compares the rendered value).
  path?: string;
  equals?: string;
}

/** A blank, immediately-valid draft of the default type between two nodes. */
export function blankEdgeDraft(source = '', target = ''): EdgeDraft {
  return { source, target, type: DEFAULT_EDGE_TYPE };
}

/**
 * Validate a single draft. Returns null when the draft is a legal edge, else a
 * human-readable reason. Mirrors the design-mode edge gate so surgery edges meet
 * the same contract the daemon enforces.
 */
export function edgeDraftError(draft: EdgeDraft): string | null {
  if (!draft.source.trim()) return 'edge needs a source node';
  if (!draft.target.trim()) return 'edge needs a target node';
  if (draft.source === draft.target) return 'edge source and target must differ';
  if (!draft.type) return 'edge type is required';
  if (!SURGERY_EDGE_TYPES.includes(draft.type)) {
    return `unknown edge type "${draft.type}"`;
  }
  if (draft.type === 'conditional') {
    if (!draft.path?.trim()) return 'conditional edge needs a path';
    if (draft.equals === undefined || draft.equals.trim() === '') {
      return 'conditional edge needs an equals value';
    }
  }
  return null;
}

/**
 * Convert a VALID draft into the wire `PipelineEdge`. Returns null when the
 * draft is invalid (callers should gate on `edgeDraftError` first; this re-checks
 * so it can never emit a malformed edge). Conditional fields are included only
 * for the `conditional` type.
 */
export function toPipelineEdge(draft: EdgeDraft): PipelineEdge | null {
  if (edgeDraftError(draft)) return null;
  const edge: PipelineEdge = {
    source: draft.source,
    target: draft.target,
    type: draft.type,
  };
  if (draft.type === 'conditional') {
    edge.path = draft.path?.trim();
    edge.equals = draft.equals;
  }
  return edge;
}

/**
 * Build the full edge list from drafts. Returns either the edges or the FIRST
 * draft error (with its index), so the dialog can refuse to submit and point at
 * the offending row.
 */
export function buildEdges(
  drafts: readonly EdgeDraft[]
): { edges: PipelineEdge[] } | { error: string; index: number } {
  const edges: PipelineEdge[] = [];
  for (let index = 0; index < drafts.length; index += 1) {
    const reason = edgeDraftError(drafts[index]);
    if (reason) return { error: reason, index };
    const edge = toPipelineEdge(drafts[index]);
    if (!edge) return { error: 'invalid edge', index };
    edges.push(edge);
  }
  return { edges };
}

/**
 * Node-id collisions between two id sets, sorted. Empty ⇒ no collision. Used to
 * preflight graft (new nodes vs existing) and join (source vs target).
 */
export function collisions(
  targetIds: readonly string[],
  sourceIds: readonly string[]
): string[] {
  const target = new Set(targetIds);
  const hits = new Set<string>();
  for (const id of sourceIds) {
    if (target.has(id)) hits.add(id);
  }
  return [...hits].sort();
}
