import type { Edge, Node } from '@vue-flow/core';

import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
  PipelineNodeType,
  ToolProvenance,
  ViewNode,
} from './pipeline-types';
import {
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
} from './pipeline-types';
import { createNode } from './pipeline-node-factory';

export type FlowNodeData = {
  nodeId: string;
  label: string;
  type: PipelineNodeType;
  acpSessionId?: string;
  thinkingSessionId?: string;
  workflowId?: string;
  acpClient?: string;
  prompt?: string;
  agent?: string;
  topic?: string;
  totalThoughts?: number;
  analysisType?: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  toolProvenance?: ToolProvenance | null;
  status?: ViewNode['status'];
  attempts?: number;
};

function positionCacheKey(pipelineId?: string): string {
  return `pipeline-positions-v3-${pipelineId ?? 'design'}`;
}

export function loadPositions(
  pipelineId?: string,
): Record<string, { x: number; y: number }> {
  try {
    const raw = localStorage.getItem(positionCacheKey(pipelineId));
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

export function savePositions(
  flowNodes: Node[],
  pipelineId?: string,
): void {
  const positions: Record<string, { x: number; y: number }> = {};
  for (const node of flowNodes) {
    positions[node.id] = { x: node.position.x, y: node.position.y };
  }
  try {
    localStorage.setItem(positionCacheKey(pipelineId), JSON.stringify(positions));
  } catch {
    // Ignore localStorage failures.
  }
}

function defaultPosition(index: number): { x: number; y: number } {
  return { x: (index % 4) * 340 + 40, y: Math.floor(index / 4) * 240 + 40 };
}

function toFlowNodeData(
  node: PipelineNode,
  viewNode?: ViewNode,
  nodeLabels?: Record<string, string>,
): FlowNodeData {
  const base = {
    nodeId: node.id,
    label: nodeLabels?.[node.id] ?? node.id,
    type: node.type ?? 'workflow',
    acpSessionId: viewNode?.acpSessionId,
    thinkingSessionId: viewNode?.thinkingSessionId,
    status: viewNode?.status,
    attempts: viewNode?.attempts,
  };

  if (isWorkflowNode(node)) {
    return {
      ...base,
      workflowId: node.workflowId,
      ...(node.acpClient ? { acpClient: node.acpClient } : {}),
      ...(node.prompt ? { prompt: node.prompt } : {}),
    };
  }

  if (node.type === 'prompt') {
    return {
      ...base,
      agent: node.agent,
      prompt: node.prompt,
    };
  }

  if (isSequentialThinkingNode(node)) {
    return {
      ...base,
      topic: node.topic,
      agent: node.agent,
      ...(typeof node.totalThoughts === 'number'
        ? { totalThoughts: node.totalThoughts }
        : {}),
      ...(node.analysisType ? { analysisType: node.analysisType } : {}),
    };
  }

  if (isToolNode(node)) {
    return {
      ...base,
      ...(node.agent ? { agent: node.agent } : {}),
      toolName: node.toolName,
      ...(node.toolArgs ? { toolArgs: node.toolArgs } : {}),
      ...(viewNode?.type === 'tool' && viewNode.toolProvenance
        ? {
            toolProvenance: {
              toolName: viewNode.toolProvenance.toolName,
              ...(viewNode.toolProvenance.resolvedArgs
                ? { resolvedArgs: { ...viewNode.toolProvenance.resolvedArgs } }
                : {}),
              ...(viewNode.toolProvenance.responseSummary != null
                ? { responseSummary: viewNode.toolProvenance.responseSummary }
                : {}),
            },
          }
        : {}),
    };
  }

  const unreachable: never = node;
  throw new Error(`Unsupported pipeline node type: ${JSON.stringify(unreachable)}`);
}

function toPipelineNode(data: FlowNodeData): PipelineNode {
  switch (data.type) {
    case 'prompt':
      return createNode('prompt', {
        id: data.nodeId,
        prompt: data.prompt ?? '',
        agent: data.agent ?? '',
      });
    case 'sequentialThinking':
      return createNode('sequentialThinking', {
        id: data.nodeId,
        topic: data.topic ?? '',
        agent: data.agent ?? '',
        ...(typeof data.totalThoughts === 'number'
          ? { totalThoughts: data.totalThoughts }
          : {}),
        ...(data.analysisType ? { analysisType: data.analysisType } : {}),
      });
    case 'tool':
      return createNode('tool', {
        id: data.nodeId,
        toolName: data.toolName ?? '',
        ...(data.agent ? { agent: data.agent } : {}),
        ...(data.toolArgs ? { toolArgs: data.toolArgs } : {}),
      });
    default:
      return createNode('workflow', {
        id: data.nodeId,
        workflowId: data.workflowId ?? '',
        agent: data.agent ?? data.acpClient ?? '',
        ...(data.prompt ? { prompt: data.prompt } : {}),
      });
  }
}

export function toPipelineNodeFromViewNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt':
      return createNode('prompt', {
        id: node.nodeId,
        prompt: node.prompt,
        agent: node.agent,
      });
    case 'sequentialThinking':
      return createNode('sequentialThinking', {
        id: node.nodeId,
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number'
          ? { totalThoughts: node.totalThoughts }
          : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
      });
    case 'tool':
      return createNode('tool', {
        id: node.nodeId,
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: node.toolArgs } : {}),
      });
    default:
      return createNode('workflow', {
        id: node.nodeId,
        workflowId: node.workflowId,
        agent: node.acpClient ?? '',
        ...(node.prompt ? { prompt: node.prompt } : {}),
      });
  }
}

export function pipelineNodesToFlow(
  pipelineNodes: PipelineNode[],
  pipelineId?: string,
  viewNodes?: ViewNode[],
  nodeLabels?: Record<string, string>,
): Node[] {
  const cached = loadPositions(pipelineId);
  const viewMap = new Map(viewNodes?.map((node) => [node.nodeId, node]) ?? []);
  return pipelineNodes.map((node, index) => ({
    id: node.id,
    type: 'pipelineNode',
    position: cached[node.id] ?? defaultPosition(index),
    data: toFlowNodeData(node, viewMap.get(node.id), nodeLabels),
  }));
}

export function pipelineEdgesToFlow(pipelineEdges: PipelineEdge[]): Edge[] {
  return pipelineEdges.map((edge) => ({
    id: `${edge.source}->${edge.target}`,
    source: edge.source,
    target: edge.target,
    type: 'smoothstep',
    animated: edge.type === 'always',
    label: edge.type ?? 'success',
    data: {
      edgeType: edge.type ?? 'success',
      ...(edge.mapping ? { mapping: { ...edge.mapping } } : {}),
    },
  }));
}

export function definitionFromFlow(
  flowNodes: Node[],
  flowEdges: Edge[],
): PipelineDefinition {
  return {
    nodes: flowNodes.map((node) => toPipelineNode(node.data as FlowNodeData)),
    edges: flowEdges.map((edge) => ({
      source: edge.source,
      target: edge.target,
      type: (edge.data?.edgeType as 'success' | 'always') ?? 'success',
      ...(() => {
        if (!edge.data?.mapping || typeof edge.data.mapping !== 'object' || Array.isArray(edge.data.mapping)) {
          return {};
        }

        const mapping = Object.fromEntries(
          Object.entries(edge.data.mapping as Record<string, unknown>).filter(
            (entry): entry is [string, string] => typeof entry[1] === 'string',
          ),
        );

        return Object.keys(mapping).length > 0 ? { mapping } : {};
      })(),
    })),
  };
}
