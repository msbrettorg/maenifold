/**
 * Pipeline template runtime — interpolation, agent resolution, and
 * capability gating logic.
 *
 * All functions are pure (no Vue reactivity) so they can be tested and
 * consumed from stores without coupling to component lifecycle.
 */
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
} from './pipeline-types';
import { getNodeType } from './pipeline-types';
import type {
  PipelineTemplate,
  TemplateVariables,
} from './pipeline-templates';
import { createNodeId } from './pipeline-node-factory';
import { PIPELINE_TEMPLATES } from './pipeline-templates';

// ---------------------------------------------------------------------------
// Agent resolution
// ---------------------------------------------------------------------------

export interface ResolvedAgents {
  primaryAgent: string;
  criticAgent: string;
}

/**
 * Deterministic agent resolution:
 * - 0 agents → both empty (safe for gallery display; nodes need manual assignment)
 * - 1 agent  → primary === critic (same agent fills both roles)
 * - 2+ agents → primary = preferred configured agent when present, otherwise first
 *               configured; critic = first distinct from primary
 */
export function resolveAgents(
  agentNames: readonly string[],
  primaryPreference?: string,
): ResolvedAgents {
  if (agentNames.length === 0) {
    return { primaryAgent: '', criticAgent: '' };
  }

  const preferred = primaryPreference?.trim();
  const primary =
    preferred && agentNames.includes(preferred) ? preferred : agentNames[0];

  if (agentNames.length === 1) {
    return { primaryAgent: primary, criticAgent: primary };
  }

  const critic = agentNames.find((name) => name !== primary) ?? primary;
  return { primaryAgent: primary, criticAgent: critic };
}

// ---------------------------------------------------------------------------
// Capability gating
// ---------------------------------------------------------------------------

/**
 * Returns true when every capability gate on the template is satisfied by
 * the set of discovered MCP tool names.
 */
export function isTemplateAvailable(
  template: PipelineTemplate,
  discoveredToolNames: ReadonlySet<string>,
): boolean {
  if (template.capabilities.length === 0) {
    return true;
  }

  return template.capabilities.every((gate) =>
    discoveredToolNames.has(gate.toolName),
  );
}

/**
 * Filters the full template registry down to those whose capability gates
 * are satisfied by the discovered tools.
 */
export function getAvailableTemplates(
  discoveredToolNames: ReadonlySet<string>,
): readonly PipelineTemplate[] {
  return PIPELINE_TEMPLATES.filter((template) =>
    isTemplateAvailable(template, discoveredToolNames),
  );
}

// ---------------------------------------------------------------------------
// Interpolation
// ---------------------------------------------------------------------------

const VARIABLE_PATTERN = /\{\{\s*([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?)\s*\}\}/g;

/** Interpolate `{{key}}` and `{{inputs.key}}` placeholders in a string. */
function interpolateString(
  text: string,
  variables: TemplateVariables,
): string {
  return text.replace(VARIABLE_PATTERN, (match, key: string) => {
    if (key === 'primaryAgent' || key === 'criticAgent') {
      return variables[key];
    }

    if (key.startsWith('inputs.')) {
      const inputKey = key.slice('inputs.'.length);
      return variables.inputs?.[inputKey] ?? match;
    }

    return match;
  });
}

/**
 * Recursively walk a JSON-safe value and interpolate all string leaves.
 * Objects and arrays are shallow-cloned so the original template is untouched.
 */
function interpolateValue(
  value: unknown,
  variables: TemplateVariables,
): unknown {
  if (typeof value === 'string') {
    return interpolateString(value, variables);
  }

  if (Array.isArray(value)) {
    return value.map((item) => interpolateValue(item, variables));
  }

  if (typeof value === 'object' && value !== null) {
    const result: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(value)) {
      result[key] = interpolateValue(val, variables);
    }
    return result;
  }

  return value;
}

function interpolateNode(
  node: PipelineNode,
  variables: TemplateVariables,
): PipelineNode {
  return interpolateValue(node, variables) as PipelineNode;
}

function interpolateEdge(
  edge: PipelineEdge,
  variables: TemplateVariables,
): PipelineEdge {
  return interpolateValue(edge, variables) as PipelineEdge;
}

// ---------------------------------------------------------------------------
// Template instantiation
// ---------------------------------------------------------------------------

export interface InstantiateResult {
  definition: PipelineDefinition;
}

/**
 * Instantiate a template by interpolating variables and cloning the
 * definition. Authoring-time template labels are stamped onto each runtime
 * node as `displayLabel`.
 */
export function instantiateTemplate(
  template: PipelineTemplate,
  variables: TemplateVariables,
): InstantiateResult {
  const idMap = new Map<string, string>();
  const nodes = template.definition.nodes.map((node) => {
    const interpolatedNode = interpolateNode(node, variables);
    const newId = createNodeId(getNodeType(node));
    idMap.set(node.id, newId);
    const displayLabel = template.nodeLabels[node.id];
    const next = { ...interpolatedNode, id: newId };

    return displayLabel
      ? { ...next, displayLabel }
      : next;
  });
  const edges = template.definition.edges.map((edge) => {
    const remapped = interpolateEdge(edge, variables);
    return {
      ...remapped,
      source: idMap.get(remapped.source) ?? remapped.source,
      target: idMap.get(remapped.target) ?? remapped.target,
    };
  });

  return {
    definition: {
      nodes,
      edges,
    },
  };
}
