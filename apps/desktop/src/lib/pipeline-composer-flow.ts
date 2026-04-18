import type { Edge, Node } from '@vue-flow/core';

import type {
  EdgeCondition,
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
  PipelineNodeAgentConfig,
  PipelineNodeType,
  ToolProvenance,
  ViewNode,
} from './pipeline-types';
import {
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
} from './pipeline-types';
import { getNodeShellClassName, getNodeShellStyle } from './pipeline-node-status';
import { createNode } from './pipeline-node-factory';

export type FlowNodeData = {
  nodeId: string;
  displayLabel: string;
  type: PipelineNodeType;
  model?: string;
  /** Full agentConfig for workflow nodes — carries mode, thoughtLevel, configOptions. */
  agentConfig?: PipelineNodeAgentConfig;
  acpSessionId?: string;
  thinkingSessionId?: string;
  workflowId?: string;
  acpClient?: string;
  prompt?: string;
  agent?: string;
  topic?: string;
  totalThoughts?: number;
  thinkingTraceLength?: number;
  analysisType?: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  toolProvenance?: ToolProvenance | null;
  status?: ViewNode['status'];
  attempts?: number;
  failureReason?: string | null;
};

function getNodeModel(node: PipelineNode): string | undefined {
  const fromAgentConfig = 'agentConfig' in node ? node.agentConfig?.model : undefined;
  return fromAgentConfig ?? ('model' in node ? node.model : undefined);
}

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
): FlowNodeData {
  const base = {
    nodeId: node.id,
    displayLabel: viewNode?.displayLabel ?? node.displayLabel ?? node.id,
    type: node.type ?? 'workflow',
    ...(getNodeModel(node) ? { model: getNodeModel(node) } : {}),
    acpSessionId: viewNode?.acpSessionId,
    thinkingSessionId: viewNode?.thinkingSessionId,
    status: viewNode?.status,
    attempts: viewNode?.attempts,
    failureReason: viewNode?.failureReason,
  };

  if (isWorkflowNode(node)) {
    return {
      ...base,
      workflowId: node.workflowId,
      ...(node.acpClient ? { acpClient: node.acpClient } : {}),
      ...(node.prompt ? { prompt: node.prompt } : {}),
      // Preserve full agentConfig so mode/thoughtLevel/configOptions survive the
      // PipelineNode → FlowNodeData → PipelineNode round-trip.
      ...(node.agentConfig ? { agentConfig: { ...node.agentConfig, ...(node.agentConfig.configOptions ? { configOptions: { ...node.agentConfig.configOptions } } : {}) } } : {}),
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
      ...(viewNode?.type === 'sequentialThinking'
        ? { thinkingTraceLength: viewNode.thinking_trace?.length ?? 0 }
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
        ...(data.displayLabel && data.displayLabel !== data.nodeId
          ? { displayLabel: data.displayLabel }
          : {}),
        prompt: data.prompt ?? '',
        agent: data.agent ?? '',
        ...(data.model ? { model: data.model } : {}),
      });
    case 'sequentialThinking':
      return createNode('sequentialThinking', {
        id: data.nodeId,
        ...(data.displayLabel && data.displayLabel !== data.nodeId
          ? { displayLabel: data.displayLabel }
          : {}),
        topic: data.topic ?? '',
        agent: data.agent ?? '',
        ...(data.model ? { model: data.model } : {}),
        ...(typeof data.totalThoughts === 'number'
          ? { totalThoughts: data.totalThoughts }
          : {}),
        ...(data.analysisType ? { analysisType: data.analysisType } : {}),
      });
    case 'tool':
      return createNode('tool', {
        id: data.nodeId,
        ...(data.displayLabel && data.displayLabel !== data.nodeId
          ? { displayLabel: data.displayLabel }
          : {}),
        toolName: data.toolName ?? '',
        ...(data.agent ? { agent: data.agent } : {}),
        ...(data.model ? { model: data.model } : {}),
        ...(data.toolArgs ? { toolArgs: data.toolArgs } : {}),
      });
    default:
      return createNode('workflow', {
        id: data.nodeId,
        ...(data.displayLabel && data.displayLabel !== data.nodeId
          ? { displayLabel: data.displayLabel }
          : {}),
        workflowId: data.workflowId ?? '',
        acpClient: data.acpClient ?? '',
        ...(data.model ? { model: data.model } : {}),
        // Restore agentConfig so mode/thoughtLevel/configOptions are not silently dropped.
        ...(data.agentConfig ? { agentConfig: data.agentConfig } : {}),
        ...(data.prompt ? { prompt: data.prompt } : {}),
      });
  }
}

export function toPipelineNodeFromViewNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt':
      return createNode('prompt', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.agentConfig?.model ?? node.model ? { model: node.agentConfig?.model ?? node.model } : {}),
      });
    case 'sequentialThinking':
      return createNode('sequentialThinking', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(node.agentConfig?.model ?? node.model ? { model: node.agentConfig?.model ?? node.model } : {}),
        ...(typeof node.totalThoughts === 'number'
          ? { totalThoughts: node.totalThoughts }
          : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
      });
    case 'tool':
      return createNode('tool', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.agentConfig?.model ?? node.model ? { model: node.agentConfig?.model ?? node.model } : {}),
        ...(node.toolArgs ? { toolArgs: node.toolArgs } : {}),
      });
    default:
      return createNode('workflow', {
        id: node.nodeId,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        acpClient: node.acpClient ?? '',
        // Merge flat model/thoughtLevel with agentConfig, preferring agentConfig values.
        // This ensures mode, thoughtLevel, and configOptions are not dropped when the
        // view node is converted back to a pipeline node definition.
        ...((): { agentConfig?: PipelineNodeAgentConfig } => {
          const effectiveModel = (node.agentConfig?.model?.length ? node.agentConfig.model : undefined) ?? (node.model?.length ? node.model : undefined);
          const effectiveMode = node.agentConfig?.mode?.length ? node.agentConfig.mode : undefined;
          const effectiveThoughtLevel = (node.agentConfig?.thoughtLevel?.length ? node.agentConfig.thoughtLevel : undefined) ?? (node.thoughtLevel?.length ? node.thoughtLevel : undefined);
          const effectiveConfigOptions = node.agentConfig?.configOptions;
          const merged: PipelineNodeAgentConfig = {
            ...(effectiveModel ? { model: effectiveModel } : {}),
            ...(effectiveMode ? { mode: effectiveMode } : {}),
            ...(effectiveThoughtLevel ? { thoughtLevel: effectiveThoughtLevel } : {}),
            ...(effectiveConfigOptions && Object.keys(effectiveConfigOptions).length > 0 ? { configOptions: { ...effectiveConfigOptions } } : {}),
          };
          return Object.keys(merged).length > 0 ? { agentConfig: merged } : {};
        })(),
        ...(node.prompt ? { prompt: node.prompt } : {}),
      });
  }
}

export function pipelineNodesToFlow(
  pipelineNodes: PipelineNode[],
  pipelineId?: string,
  viewNodes?: ViewNode[],
): Node[] {
  const cached = loadPositions(pipelineId);
  const viewMap = new Map(viewNodes?.map((node) => [node.nodeId, node]) ?? []);
  return pipelineNodes.map((node, index) => ({
    id: node.id,
    type: 'pipelineNode',
    class: getNodeShellClassName(viewMap.get(node.id)?.status),
    position: cached[node.id] ?? defaultPosition(index),
    style: getNodeShellStyle(viewMap.get(node.id)?.status),
    data: toFlowNodeData(node, viewMap.get(node.id)),
  }));
}

export function pipelineEdgesToFlow(pipelineEdges: PipelineEdge[]): Edge[] {
  const edgeLabel = (edge: PipelineEdge): string =>
    edge.type === 'conditional' && edge.condition
      ? `?{${edge.condition.path}}=${JSON.stringify(edge.condition.equals)}`
      : edge.type ?? 'success';

  return pipelineEdges.map((edge) => ({
    id: `${edge.source}->${edge.target}`,
    source: edge.source,
    target: edge.target,
    type: 'smoothstep',
    animated: edge.type === 'always',
    label: edgeLabel(edge),
    data: {
      edgeType: edge.type ?? 'success',
      ...(edge.condition ? { condition: { ...edge.condition } } : {}),
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
      type: (edge.data?.edgeType as 'success' | 'always' | 'conditional') ?? 'success',
      ...(() => {
        const condition = edge.data?.condition as Partial<EdgeCondition> | undefined;
        const path = typeof condition?.path === 'string' ? condition.path : '';
        const equals = condition?.equals;
        if (
          path.length > 0 &&
          (typeof equals === 'string' || typeof equals === 'number' || typeof equals === 'boolean')
        ) {
          return { condition: { path, equals } };
        }
        return {};
      })(),
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
