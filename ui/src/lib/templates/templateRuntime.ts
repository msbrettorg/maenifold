// Pipeline template runtime — pure interpolation, agent resolution, and
// capability gating. Framework-free so it is unit-testable without a DOM, a
// store, or Vue reactivity, and consumable from both components and tests.
//
// Ported from the old Electron UI's pipeline-template-runtime.ts and adapted to:
//   - the current `pipeline-types.ts` node/edge shapes,
//   - fresh ids via `uniqueNodeId(crypto.randomUUID)` from `pipelineDesignOps.ts`
//     (NOT the removed `createNodeId`),
//   - full field preservation (type/mode/bindings/mapping/path/equals) through a
//     recursive interpolation walk.

import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
  PromptNode,
} from '../pipeline-types';
import { uniqueNodeId } from '../pipelineDesignOps';
import type {
  PipelineTemplate,
  TemplateInputValues,
  TemplateVariables,
} from './pipelineTemplates';

// ---------------------------------------------------------------------------
// Agent resolution
// ---------------------------------------------------------------------------

export interface ResolvedAgents {
  primaryAgent: string;
  criticAgent: string;
}

/**
 * Deterministic agent resolution:
 * - 0 agents  → both empty (gallery still renders; create is blocked upstream)
 * - 1 agent   → primary === critic (the single agent self-critiques)
 * - 2+ agents → primary = preferred (when present & valid) else first;
 *               critic  = first agent distinct from primary
 */
export function resolveAgents(
  agentNames: readonly string[],
  primaryPreference?: string
): ResolvedAgents {
  if (agentNames.length === 0) {
    return { primaryAgent: '', criticAgent: '' };
  }

  const preferred = primaryPreference?.trim();
  const primary = preferred && agentNames.includes(preferred) ? preferred : agentNames[0];

  if (agentNames.length === 1) {
    return { primaryAgent: primary, criticAgent: primary };
  }

  const critic = agentNames.find((name) => name !== primary) ?? primary;
  return { primaryAgent: primary, criticAgent: critic };
}

// ---------------------------------------------------------------------------
// Capability gating
// ---------------------------------------------------------------------------

/** Discovery lifecycle of the MCP tool set used to gate enterprise templates. */
export type ToolDiscoveryStatus = 'idle' | 'loading' | 'ready' | 'error';

export type TemplateAvailability = 'available' | 'unavailable' | 'discovery-pending';

/**
 * True when every capability gate on the template is satisfied by the set of
 * discovered MCP tool names. Universal (no-capability) templates are always
 * available. Multi-tool templates require ALL listed tools to be present.
 */
export function isTemplateAvailable(
  template: PipelineTemplate,
  discoveredToolNames: ReadonlySet<string>
): boolean {
  if (template.capabilities.length === 0) return true;
  return template.capabilities.every((gate) => discoveredToolNames.has(gate.toolName));
}

/**
 * Capability gate that FAILS CLOSED for enterprise templates while tool
 * discovery is still loading or has failed: an enterprise template is only ever
 * 'available' once discovery is 'ready' AND all its required tools are present.
 * Universal templates are always 'available' regardless of discovery status.
 */
export function templateAvailability(
  template: PipelineTemplate,
  discoveredToolNames: ReadonlySet<string>,
  status: ToolDiscoveryStatus
): TemplateAvailability {
  if (template.capabilities.length === 0) return 'available';
  if (status !== 'ready') return 'discovery-pending';
  return isTemplateAvailable(template, discoveredToolNames) ? 'available' : 'unavailable';
}

/** Universal templates always; enterprise templates only when fully available. */
export function getAvailableTemplates(
  templates: readonly PipelineTemplate[],
  discoveredToolNames: ReadonlySet<string>,
  status: ToolDiscoveryStatus
): readonly PipelineTemplate[] {
  return templates.filter(
    (template) => templateAvailability(template, discoveredToolNames, status) === 'available'
  );
}

// ---------------------------------------------------------------------------
// Interpolation
// ---------------------------------------------------------------------------

// Matches `{{primaryAgent}}`, `{{criticAgent}}`, and `{{inputs.someKey}}`.
const VARIABLE_PATTERN = /\{\{\s*([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?)\s*\}\}/g;

/** Interpolate `{{key}}` / `{{inputs.key}}` placeholders in a single string. */
function interpolateString(text: string, variables: TemplateVariables): string {
  return text.replace(VARIABLE_PATTERN, (match, key: string) => {
    if (key === 'primaryAgent') return variables.primaryAgent;
    if (key === 'criticAgent') return variables.criticAgent;
    if (key.startsWith('inputs.')) {
      const inputKey = key.slice('inputs.'.length);
      return variables.inputs?.[inputKey] ?? match;
    }
    return match;
  });
}

/**
 * Recursively walk a JSON-safe value, interpolating every string leaf. Objects
 * and arrays are rebuilt so the original template is never mutated. This is what
 * preserves nested structures such as a tool node's `toolArgs` and any array
 * values (e.g. `allowedTools`).
 */
export function interpolateValue(value: unknown, variables: TemplateVariables): unknown {
  if (typeof value === 'string') return interpolateString(value, variables);
  if (Array.isArray(value)) return value.map((item) => interpolateValue(item, variables));
  if (typeof value === 'object' && value !== null) {
    const result: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(value)) {
      result[key] = interpolateValue(val, variables);
    }
    return result;
  }
  return value;
}

// ---------------------------------------------------------------------------
// Template instantiation
// ---------------------------------------------------------------------------

export interface InstantiateResult {
  definition: PipelineDefinition;
  /** authoring id → fresh runtime id (exposed for diagnostics/tests). */
  idMap: ReadonlyMap<string, string>;
}

/**
 * Instantiate a template into a concrete `PipelineDefinition`:
 *   1. interpolate every node/edge string leaf (recursively — preserves
 *      `toolArgs`, arrays, and all flat fields: type/mode/bindings/mapping/
 *      path/equals),
 *   2. assign a fresh id per node via `uniqueNodeId(crypto.randomUUID)`,
 *   3. stamp the authoring `nodeLabels` onto each node as `displayLabel`,
 *   4. remap every edge endpoint to the fresh ids.
 *
 * Edge endpoints that don't resolve to a fresh id are left UNCHANGED so the
 * downstream `validateDefinition` gate (run before create) fails hard on an
 * unmapped endpoint rather than this function silently dropping it.
 */
export function instantiateTemplate(
  template: PipelineTemplate,
  variables: TemplateVariables
): InstantiateResult {
  const idMap = new Map<string, string>();
  const assigned: PipelineNode[] = [];

  const nodes = template.definition.nodes.map((node) => {
    const interpolated = interpolateValue(node, variables) as PipelineNode;
    // Fresh id, unique among the nodes already assigned in THIS instantiation.
    const newId = uniqueNodeId(assigned);
    idMap.set(node.id, newId);
    const displayLabel = template.nodeLabels[node.id];
    const next: PipelineNode = displayLabel
      ? { ...interpolated, id: newId, displayLabel }
      : { ...interpolated, id: newId };
    assigned.push(next);
    return next;
  });

  const edges: PipelineEdge[] = template.definition.edges.map((edge) => {
    const remapped = interpolateValue(edge, variables) as PipelineEdge;
    return {
      ...remapped,
      source: idMap.get(remapped.source) ?? remapped.source,
      target: idMap.get(remapped.target) ?? remapped.target,
    };
  });

  return { definition: { nodes, edges }, idMap };
}

// ---------------------------------------------------------------------------
// Intent track — a single free-form prompt node
// ---------------------------------------------------------------------------

/**
 * Build a one-node pipeline from a free-form intent string. The single prompt
 * node carries the user's intent verbatim and the resolved primary agent; the
 * id is fresh via `uniqueNodeId`.
 */
export function buildIntentDefinition(intent: string, agent: string): PipelineDefinition {
  const node: PromptNode = {
    type: 'prompt',
    id: uniqueNodeId([]),
    prompt: intent,
    agent,
    displayLabel: 'Intent',
  };
  return { nodes: [node], edges: [] };
}

/** A concise pipeline name derived from a free-form intent (first line, clipped). */
export function intentPipelineName(intent: string): string {
  const firstLine = intent.trim().split(/\r?\n/, 1)[0]?.trim() ?? '';
  if (firstLine.length === 0) return 'Intent pipeline';
  return firstLine.length > 60 ? `${firstLine.slice(0, 57)}…` : firstLine;
}

/**
 * Sanitize a free-form name into a backend-valid pipeline slug.
 *
 * Backend rule (handler_crud.rs): `^[A-Za-z][A-Za-z0-9-]*$`, max 64 chars.
 * - Collapse every run of non-alphanumeric ASCII characters into a single `-`.
 * - Strip leading characters until the first ASCII letter (slug must start with a letter).
 * - Strip a trailing `-`.
 * - Clip to 64 characters (then re-strip any trailing `-` left by the clip).
 * - If nothing valid remains, return `fallback` (assumed already valid).
 */
export function slugifyPipelineName(raw: string, fallback: string): string {
  const collapsed = raw
    .replace(/[^A-Za-z0-9]+/g, '-') // non-alphanumeric runs -> single dash
    .replace(/^[^A-Za-z]+/, '') // must start with an ASCII letter
    .replace(/-+$/, ''); // no trailing dash
  if (collapsed.length === 0) return fallback;
  const clipped = collapsed.slice(0, 64).replace(/-+$/, '');
  return clipped.length === 0 ? fallback : clipped;
}

/** Collect missing required inputs for a template against the supplied values. */
export function missingRequiredInputs(
  template: PipelineTemplate,
  values: TemplateInputValues
): string[] {
  return template.runInputs
    .filter((input) => input.required && !(values[input.key] ?? '').trim())
    .map((input) => input.key);
}
