import type {
  PipelineDefinition,
  PipelineListItem,
  PipelineView,
  PipelineNode,
  ViewNode,
} from '../lib/pipeline-types';
interface PipelineListResult {
  pipelines?: PipelineListItem[];
}

interface PipelineCreateResult {
  pipelineId?: string;
}

export function normalizePipelineList(
  result: PipelineListItem[] | PipelineListResult | undefined,
): PipelineListItem[] {
  return Array.isArray(result) ? result : result?.pipelines ?? [];
}

export function isCreateResult(value: unknown): value is PipelineCreateResult {
  return typeof value === 'object' && value !== null && 'pipelineId' in value;
}

function normalizeViewNode(node: ViewNode): ViewNode {
  return node.type === undefined ? { ...node, type: 'workflow' } : node;
}

export function normalizeView(view: PipelineView): PipelineView {
  return {
    ...view,
    nodes: view.nodes.map(normalizeViewNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}

function toLocalNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt':
      return {
        id: node.nodeId,
        type: 'prompt',
        prompt: node.prompt,
        agent: node.agent,
      };
    case 'sequentialThinking':
      return {
        id: node.nodeId,
        type: 'sequentialThinking',
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
      };
    case 'workflow':
    case undefined:
      return {
        id: node.nodeId,
        type: 'workflow',
        workflowId: node.workflowId,
        ...(node.acpClient ? { acpClient: node.acpClient } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
      };
    case 'tool':
      return {
        id: node.nodeId,
        type: 'tool',
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: node.toolArgs } : {}),
      };
  }
}

export function toLocalDefinition(view: PipelineView): PipelineDefinition {
  return {
    nodes: view.nodes.map(toLocalNode),
    edges: view.edges.map((edge) => ({ ...edge })),
  };
}
