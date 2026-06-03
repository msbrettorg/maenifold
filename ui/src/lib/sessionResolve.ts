// Live ACP inspection — node → session/agent/model resolution + inspection guard
// (T-600.7).
//
// Pure, framework-free logic shared by the pipelineInspection store and its unit
// tests. Three jobs:
//   1. resolveNodeSessionId — the canonical ACP sessionId for a running node, in
//      the priority order the task mandates: provenance.detail.sessionId →
//      acpSessionId → sessionRef.sessionId.
//   2. resolveNodeAgent / resolveNodeModel — the agent name and model that MUST
//      be known before we claim a node is inspectable.
//   3. inspectionGuard — requires sessionId AND agent AND model. When any is
//      missing it names the field, which the UI surfaces as a HIDDEN-INFERENCE
//      error: the operator must always be able to see that maenifold is reasoning
//      through a session they can inspect, never silently.
//
// Single field-name convention: the store already normalizes snake→camel once at
// the gateway boundary (stores/pipelines.ts), so there are NO `x ?? x_snake`
// dual-read fallbacks here.

import {
  viewNodeType,
  type PipelineNodeType,
  type ViewNode,
} from './pipeline-types';

// The 30-second cold-start grace window. When a node first becomes active its
// ACP sessionId may not be persisted yet; we allow up to this long for the
// sessionId/agent/model to appear before treating a still-incomplete node as a
// hidden-inference error.
export const INSPECTION_GRACE_MS = 30_000;

// Tool nodes are NOT inspected as ACP sessions (they are a single MCP tool call,
// not a reasoning agent). Everything else (prompt / workflow / sequentialThinking)
// runs a user-visible CLI agent we can observe.
export function isInspectableNodeType(type: PipelineNodeType): boolean {
  return type !== 'tool';
}

export function isInspectableNode(node: ViewNode): boolean {
  return isInspectableNodeType(viewNodeType(node));
}

// Structural reads. ViewNode is a discriminated union; rather than narrow four
// times for each field we read the optional members structurally. Every field
// read here exists on at least one ViewNode variant (see lib/pipeline-types.ts).
interface SessionBearing {
  acpSessionId?: string | null;
  sessionRef?: { agent?: string; sessionId?: string };
  acpClient?: string | null;
  agent?: string | null;
  model?: string | null;
  agentConfig?: { model?: string | null } | null;
  provenance?: { agent?: string | null; detail?: { sessionId?: string | null } };
}

function nonEmpty(value: string | null | undefined): string | null {
  return typeof value === 'string' && value.trim().length > 0 ? value : null;
}

/**
 * Resolve a node's ACP sessionId in the mandated priority order:
 *   provenance.detail.sessionId → acpSessionId → sessionRef.sessionId.
 * Returns null when none is present (the node is reasoning without a session we
 * can attach to — a hidden-inference candidate once past the grace window).
 */
export function resolveNodeSessionId(node: ViewNode): string | null {
  const n = node as SessionBearing;
  return (
    nonEmpty(n.provenance?.detail?.sessionId) ??
    nonEmpty(n.acpSessionId) ??
    nonEmpty(n.sessionRef?.sessionId)
  );
}

/**
 * Resolve the agent name driving the node. Prompt and sequentialThinking nodes
 * carry `agent` directly; workflow nodes carry `acpClient`; provenance.agent is
 * the backend-resolved fallback for any variant.
 */
export function resolveNodeAgent(node: ViewNode): string | null {
  const n = node as SessionBearing;
  return (
    nonEmpty(n.agent) ??
    nonEmpty(n.acpClient) ??
    nonEmpty(n.sessionRef?.agent) ??
    nonEmpty(n.provenance?.agent)
  );
}

/**
 * Resolve the model the node is reasoning with. `model` is the explicit field;
 * `agentConfig.model` is the per-node override. All ACP calls specify model
 * explicitly, so the guard requires it to be known.
 */
export function resolveNodeModel(node: ViewNode): string | null {
  const n = node as SessionBearing;
  return nonEmpty(n.model) ?? nonEmpty(n.agentConfig?.model);
}

export type GuardField = 'sessionId' | 'agent' | 'model';

export interface InspectionGuard {
  sessionId: string | null;
  agent: string | null;
  model: string | null;
  // The required fields that are absent, in display order. Empty ⇒ ok.
  missing: GuardField[];
  ok: boolean;
}

/**
 * The inspection guard. A node is inspectable only when sessionId, agent and
 * model are ALL present. When any is missing the field name is reported so the
 * UI can name exactly which part of the reasoning path is hidden.
 */
export function inspectionGuard(node: ViewNode): InspectionGuard {
  const sessionId = resolveNodeSessionId(node);
  const agent = resolveNodeAgent(node);
  const model = resolveNodeModel(node);
  const missing: GuardField[] = [];
  if (!sessionId) missing.push('sessionId');
  if (!agent) missing.push('agent');
  if (!model) missing.push('model');
  return { sessionId, agent, model, missing, ok: missing.length === 0 };
}

/**
 * Whether a node's grace window has elapsed. `graceStart` is the timestamp the
 * node first became active; `now` is the current clock. A node with no recorded
 * graceStart is treated as past grace (we have no warm-up basis for it).
 */
export function isPastGrace(
  graceStart: number | undefined,
  now: number,
  graceMs: number = INSPECTION_GRACE_MS
): boolean {
  if (graceStart === undefined) return true;
  return now - graceStart >= graceMs;
}

/**
 * Milliseconds remaining in the grace window (0 once elapsed). Used by the panel
 * to show a "cold-starting… (Ns)" countdown before the hidden-inference error.
 */
export function graceRemainingMs(
  graceStart: number | undefined,
  now: number,
  graceMs: number = INSPECTION_GRACE_MS
): number {
  if (graceStart === undefined) return 0;
  return Math.max(0, graceMs - (now - graceStart));
}

export interface HiddenInferenceError {
  nodeId: string;
  displayLabel: string;
  agent: string | null;
  missing: GuardField[];
}

/**
 * Compute the hidden-inference errors for the active non-tool nodes: each active
 * inspectable node whose guard fails AND whose grace window has elapsed. The
 * operator MUST see these — an active node we cannot attach to means maenifold is
 * reasoning without an inspectable session.
 */
export function hiddenInferenceErrors(
  nodes: ViewNode[],
  activeIds: ReadonlySet<string>,
  graceStartByNode: ReadonlyMap<string, number>,
  now: number,
  graceMs: number = INSPECTION_GRACE_MS
): HiddenInferenceError[] {
  const errors: HiddenInferenceError[] = [];
  for (const node of nodes) {
    if (!activeIds.has(node.nodeId)) continue;
    if (!isInspectableNode(node)) continue;
    const guard = inspectionGuard(node);
    if (guard.ok) continue;
    if (!isPastGrace(graceStartByNode.get(node.nodeId), now, graceMs)) continue;
    errors.push({
      nodeId: node.nodeId,
      displayLabel: node.displayLabel ?? node.nodeId,
      agent: guard.agent,
      missing: guard.missing,
    });
  }
  return errors;
}
