// Lossless projection between the VIEW read-model (runtime, from
// `{ sessionId, view: true }`) and the DEFINITION write-model (what the Pipeline
// `update` op persists), plus the inverse DEFINITION → VIEW projection the
// design-mode canvas renders.
//
// Why two directions:
//   - `viewToDefinition` strips runtime-only fields off a freshly loaded view so
//     the Pipelines "design mode" can edit a clean editable definition and round-
//     trip it back through `store.updatePipeline` without losing any configured
//     field (per-type config, base advanced controls, schemas, top-level
//     inputs/inputSchema/templateId).
//   - `definitionToView` re-projects that editable definition into the SAME
//     `PipelineView` shape the canvas sync (`useCanvasSync` → `canvasBridge`)
//     consumes, so add/edit/delete/duplicate mutations render live BEFORE save,
//     not only after a save+reload.
//
// The projection is generic-copy-minus-droplist rather than a hand-enumerated
// field whitelist: copying every non-runtime key (renaming `nodeId`↔`id`) is
// lossless by construction and cannot silently drop a config field the type
// model has not yet caught up to. The runtime drop-list is the authoritative
// set of fields the backend computes per run and that the definition must NOT
// carry back.

import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
  PipelineView,
  ViewNode,
} from './pipeline-types';

// Runtime-only keys the backend computes per run. These live on `ViewNode` but
// must be dropped when projecting back to a `PipelineNode` definition. Mirrors
// the field list in the T-600.4 task contract.
const RUNTIME_NODE_KEYS: ReadonlySet<string> = new Set([
  'nodeId',
  'sessionId',
  'acpSessionId',
  'thinkingSessionId',
  'provenance',
  'status',
  'attempts',
  'output',
  'inputArtifact',
  'outputArtifact',
  'failureReason',
  'reconnected',
  'stale',
  'startedAt',
  'completedAt',
  'refinement',
  'assembledPrompt',
  'workflowSteps',
  'thinkingTrace',
  'toolProvenance',
]);

// Edge fields that constitute the wire/definition shape. Identical on both the
// view and definition sides, so they are copied verbatim (never reconstructed
// from a Vue Flow edge, which may have lost `bindings`).
const EDGE_KEYS = ['source', 'target', 'type', 'path', 'equals', 'mode', 'bindings', 'mapping'] as const;

function asRecord(value: object): Record<string, unknown> {
  return value as unknown as Record<string, unknown>;
}

/**
 * Project one runtime view node into its definition node, dropping runtime-only
 * fields and renaming `nodeId` → `id`. `undefined`/`null` values are omitted so
 * the resulting definition stays terse and a round-trip is stable.
 */
export function viewNodeToDefinitionNode(node: ViewNode): PipelineNode {
  const raw = asRecord(node);
  const out: Record<string, unknown> = { id: node.nodeId };
  for (const [key, value] of Object.entries(raw)) {
    if (RUNTIME_NODE_KEYS.has(key)) continue;
    if (value === undefined || value === null) continue;
    out[key] = value;
  }
  return out as unknown as PipelineNode;
}

/** Copy an edge keeping ONLY the definition/wire fields (bindings preserved). */
export function copyDefinitionEdge(edge: PipelineEdge): PipelineEdge {
  const raw = edge as unknown as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const key of EDGE_KEYS) {
    const value = raw[key];
    // Skip both `undefined` and `null`. The daemon view emits `mapping: null`
    // (and may null other optionals); a literal `null` fails the backend schema
    // (`null is not of type "object"`), so omit the key entirely.
    if (value === undefined || value === null) continue;
    out[key] = value;
  }
  return out as unknown as PipelineEdge;
}

/** Project a runtime view into an editable, lossless pipeline definition. */
export function viewToDefinition(view: PipelineView): PipelineDefinition {
  const definition: PipelineDefinition = {
    nodes: view.nodes.map(viewNodeToDefinitionNode),
    edges: view.edges.map(copyDefinitionEdge),
  };
  if (view.inputs !== undefined) definition.inputs = view.inputs;
  if (view.inputSchema !== undefined) definition.inputSchema = view.inputSchema;
  if (view.templateId !== undefined) definition.templateId = view.templateId;
  return definition;
}

/**
 * Project a definition node back into the runtime view shape the canvas
 * consumes, synthesizing the placeholder runtime fields `canvasBridge` reads
 * (`nodeId`/`sessionId`/`status`/`attempts`). All of those are in
 * RUNTIME_NODE_KEYS, so a definition → view → definition round-trip is stable.
 */
export function definitionNodeToViewNode(node: PipelineNode): ViewNode {
  const raw = node as unknown as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(raw)) {
    if (key === 'id') continue;
    out[key] = value;
  }
  out.nodeId = node.id;
  out.sessionId = '';
  out.status = 'pending';
  out.attempts = 0;
  return out as unknown as ViewNode;
}

// The non-node metadata the design graph preserves so the canvas keeps a stable
// pipeline identity (position cache + fit latch are keyed on `pipelineId`).
export interface DesignViewMeta {
  pipelineId: string;
  name?: string | null;
  description?: string | null;
  status: PipelineView['status'];
}

/** Project an editable definition into the `PipelineView` the canvas renders. */
export function definitionToView(
  definition: PipelineDefinition,
  meta: DesignViewMeta
): PipelineView {
  return {
    pipelineId: meta.pipelineId,
    name: meta.name ?? null,
    description: meta.description ?? null,
    status: meta.status,
    inputs: definition.inputs,
    inputSchema: definition.inputSchema,
    templateId: definition.templateId,
    nodes: definition.nodes.map(definitionNodeToViewNode),
    edges: definition.edges.map(copyDefinitionEdge),
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: false,
  };
}

/**
 * The Vue Flow edge id for each definition edge, in array order. Replicates the
 * exact id scheme `canvasBridge.viewToFlowEdges` uses
 * (`${source}->${target}#${perSourceIndex}`) so a right-click on a rendered edge
 * can be resolved back to its stable array index in `definition.edges` — even
 * when parallel duplicate (source,target) edges exist.
 */
export function flowEdgeIds(edges: PipelineEdge[]): string[] {
  const sourceIndex = new Map<string, number>();
  return edges.map((edge) => {
    const idx = sourceIndex.get(edge.source) ?? 0;
    sourceIndex.set(edge.source, idx + 1);
    return `${edge.source}->${edge.target}#${idx}`;
  });
}

/** Resolve a Vue Flow edge id back to its index in `edges`, or -1 if absent. */
export function indexForFlowEdgeId(edges: PipelineEdge[], flowId: string): number {
  return flowEdgeIds(edges).indexOf(flowId);
}
