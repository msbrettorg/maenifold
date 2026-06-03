// Stateless node-form core logic (T-600.5). Extracted from the design-mode
// NodeConfigForm so BOTH design editing and surgery grafting render fields from
// ONE source of truth (lib/nodeFormFields.ts) and build/validate a node the same
// way. This module is pure (no Vue, no store): it takes a draft object plus the
// JSON-field text mirrors and returns either a validated PipelineNode or an
// error message. NodeFormCore.vue owns the reactive draft + mirrors and delegates
// build/validation here.

import {
  deletePath,
  getPath,
  setPath,
  type NodeFormFields,
} from './nodeFormFields';
import type { PipelineNode } from './pipeline-types';

export interface BuildOutcome {
  node?: PipelineNode;
  error?: string;
}

function clone(value: Record<string, unknown>): Record<string, unknown> {
  // JSON round-trip clone reads through any Vue reactive proxy (unlike
  // structuredClone, which throws on proxies in WebView2). Node config is pure
  // JSON data, so this is lossless.
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
}

/**
 * Build a node from the draft. Commits JSON fields (parsed or removed), drops
 * empty optional scalars/arrays + an empty sessionRef, then enforces required
 * fields. Returns `{ node }` on success or `{ error }` on the first failure.
 */
export function buildNodeResult(
  draft: Record<string, unknown>,
  jsonText: Record<string, string>,
  fields: NodeFormFields
): BuildOutcome {
  const result = clone(draft);
  const allFields = [...fields.required, ...fields.optional];

  // Commit JSON fields: parse the text mirror, or remove the field when blank.
  for (const field of allFields) {
    if (field.kind !== 'json') continue;
    const raw = jsonText[field.path] ?? '';
    if (!raw.trim()) {
      deletePath(result, field.path);
      continue;
    }
    try {
      setPath(result, field.path, JSON.parse(raw));
    } catch {
      return { error: `Invalid JSON in "${field.label}".` };
    }
  }

  // Drop empty optional scalars / arrays so the saved definition stays terse.
  for (const field of fields.optional) {
    if (field.kind === 'json') continue;
    const value = getPath(result, field.path);
    const isEmpty =
      value === undefined ||
      value === '' ||
      (Array.isArray(value) && value.length === 0);
    if (isEmpty) deletePath(result, field.path);
  }

  // Drop an entirely-empty sessionRef object (prompt nodes).
  const sessionRef = result.sessionRef;
  if (
    sessionRef &&
    typeof sessionRef === 'object' &&
    !(
      (sessionRef as Record<string, unknown>).agent ||
      (sessionRef as Record<string, unknown>).sessionId
    )
  ) {
    delete result.sessionRef;
  }

  // Enforce required fields (string + non-blank).
  for (const field of fields.required) {
    const value = getPath(result, field.path);
    if (typeof value !== 'string' || !value.trim()) {
      return { error: `"${field.label}" is required.` };
    }
  }

  return { node: result as unknown as PipelineNode };
}
