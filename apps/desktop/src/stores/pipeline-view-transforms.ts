// Re-export shim. Implementation split across:
//   pipeline-view-transforms-clone.ts (clone helpers + legacy field merging)
//   pipeline-view-transforms-serialize.ts (toLocalDefinition + serializePipelineDefinition)
import type { PipelineListItem, PipelineView, ViewNode } from '../lib/pipeline-types-view';
import {
  cloneAgentConfig,
  cloneInputs,
  cloneNodeAdopt,
  cloneNodeProvenance,
  cloneSessionRef,
  cloneToolProvenance,
} from './pipeline-view-transforms-clone';

export { toLocalDefinition, serializePipelineDefinition } from './pipeline-view-transforms-serialize';

interface PipelineListResult { pipelines?: PipelineListItem[]; }
interface PipelineCreateResult { pipelineId?: string; }

export function normalizePipelineList(
  result: PipelineListItem[] | PipelineListResult | undefined,
): PipelineListItem[] {
  return Array.isArray(result) ? result : result?.pipelines ?? [];
}

export function isCreateResult(value: unknown): value is PipelineCreateResult {
  return typeof value === 'object' && value !== null && 'pipelineId' in value;
}

function normalizeViewNode(node: ViewNode): ViewNode {
  const normalized = node.type === undefined ? { ...node, type: 'workflow' as const } : { ...node };
  if ('toolArgs' in normalized && normalized.toolArgs) {
    normalized.toolArgs = { ...normalized.toolArgs };
  }
  if ('inputSchema' in normalized && normalized.inputSchema) {
    normalized.inputSchema = { ...normalized.inputSchema };
  }
  if ('outputSchema' in normalized && normalized.outputSchema) {
    normalized.outputSchema = { ...normalized.outputSchema };
  }
  if ('allowedTools' in normalized && Array.isArray(normalized.allowedTools)) {
    normalized.allowedTools = [...normalized.allowedTools];
  }
  if ('agentConfig' in normalized && normalized.agentConfig) {
    normalized.agentConfig = cloneAgentConfig(normalized.agentConfig);
  }
  if ('sessionRef' in normalized && normalized.sessionRef) {
    normalized.sessionRef = cloneSessionRef(normalized.sessionRef);
  }
  if ('adopt' in normalized && normalized.adopt) {
    normalized.adopt = cloneNodeAdopt(normalized.adopt);
  }
  if ('toolProvenance' in normalized && normalized.toolProvenance) {
    normalized.toolProvenance = cloneToolProvenance(normalized.toolProvenance);
  }
  if ('provenance' in normalized && normalized.provenance) {
    normalized.provenance = cloneNodeProvenance(normalized.provenance);
  }
  return normalized;
}

export function normalizeView(view: PipelineView): PipelineView {
  return {
    ...view,
    ...(view.inputs ? { inputs: cloneInputs(view.inputs) } : {}),
    ...(view.inputSchema ? { inputSchema: { ...view.inputSchema } } : {}),
    nodes: view.nodes.map(normalizeViewNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}
