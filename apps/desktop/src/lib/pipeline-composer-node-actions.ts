import type { Edge, Node } from '@vue-flow/core';

import { createNode, createNodeId } from './pipeline-node-factory';
import { pipelineNodesToFlow } from './pipeline-composer-flow';
import type { PipelineDefinition, PipelineNode } from './pipeline-types';
import {
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
} from './pipeline-types';

type Point = { x: number; y: number };
type SelectedEdge = { source: string; target: string };

type ComposerNodeActionsOptions = {
  designMode: () => boolean;
  localDefinition: () => PipelineDefinition | undefined;
  selectNode: (nodeId: string | null) => void;
  selectEdge: (edge: SelectedEdge | null) => void;
  getNodes: () => Node[];
  getEdges: () => Edge[];
  syncLocalGraph: (nodes: Node[], edges: Edge[]) => void;
};

function cloneExecutionFields(
  node: Extract<PipelineNode, { type: 'prompt' | 'sequentialThinking' | 'tool' }>,
) {
  return {
    ...(node.model ? { model: node.model } : {}),
    ...(node.thoughtLevel ? { thoughtLevel: node.thoughtLevel } : {}),
    ...(node.allowedTools ? { allowedTools: [...node.allowedTools] } : {}),
    ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
    ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
    ...(node.agentConfig
      ? {
          agentConfig: {
            ...node.agentConfig,
            ...(node.agentConfig.configOptions
              ? { configOptions: { ...node.agentConfig.configOptions } }
              : {}),
          },
        }
      : {}),
  };
}

function cloneNodeWithNewId(node: PipelineNode): PipelineNode {
  const type = node.type ?? 'workflow';
  const id = createNodeId(type);

  if (isWorkflowNode(node)) {
    return createNode('workflow', {
      id,
      ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
      workflowId: node.workflowId,
      agent: node.acpClient ?? '',
      ...(node.prompt ? { prompt: node.prompt } : {}),
    });
  }

  if (isPromptNode(node)) {
    return createNode('prompt', {
      id,
      ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
      prompt: node.prompt,
      agent: node.agent,
      ...(node.toolMode ? { toolMode: node.toolMode } : {}),
      ...cloneExecutionFields(node),
    });
  }

  if (isSequentialThinkingNode(node)) {
    return createNode('sequentialThinking', {
      id,
      ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
      topic: node.topic,
      agent: node.agent,
      ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
      ...(node.analysisType ? { analysisType: node.analysisType } : {}),
      ...cloneExecutionFields(node),
    });
  }

  if (isToolNode(node)) {
    return createNode('tool', {
      id,
      ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
      toolName: node.toolName,
      ...(node.agent ? { agent: node.agent } : {}),
      ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
      ...cloneExecutionFields(node),
    });
  }

  const unreachable: never = node;
  throw new Error(`Unsupported node type: ${JSON.stringify(unreachable)}`);
}

export function createComposerNodeActions(options: ComposerNodeActionsOptions) {
  function findDefinitionNode(nodeId: string): PipelineNode | undefined {
    return options.localDefinition()?.nodes.find((node) => node.id === nodeId);
  }

  function createNodeAt(node: PipelineNode, position: Point): void {
    if (!options.designMode()) return;

    const nextNodes = [
      ...options.getNodes(),
      {
        id: node.id,
        type: 'pipelineNode',
        position,
        data: { ...pipelineNodesToFlow([node])[0]!.data },
      } satisfies Node,
    ];
    options.syncLocalGraph(nextNodes, options.getEdges());
    options.selectNode(node.id);
    options.selectEdge(null);
  }

  function deleteNode(nodeId: string): void {
    if (!options.designMode()) return;

    const nextNodes = options.getNodes().filter((node) => node.id !== nodeId);
    const nextEdges = options.getEdges().filter(
      (edge) => edge.source !== nodeId && edge.target !== nodeId,
    );
    options.syncLocalGraph(nextNodes, nextEdges);
    options.selectNode(null);
    options.selectEdge(null);
  }

  function disconnectNodeEdges(nodeId: string): void {
    if (!options.designMode()) return;

    const nextEdges = options.getEdges().filter(
      (edge) => edge.source !== nodeId && edge.target !== nodeId,
    );
    options.syncLocalGraph(options.getNodes(), nextEdges);
    options.selectNode(nodeId);
    options.selectEdge(null);
  }

  function duplicateNode(nodeId: string, offset = 50): void {
    if (!options.designMode()) return;

    const sourceNode = findDefinitionNode(nodeId);
    const sourceFlowNode = options.getNodes().find((node) => node.id === nodeId);
    if (!sourceNode || !sourceFlowNode) return;

    createNodeAt(cloneNodeWithNewId(sourceNode), {
      x: sourceFlowNode.position.x + offset,
      y: sourceFlowNode.position.y + offset,
    });
  }

  function editNode(nodeId: string): void {
    options.selectNode(nodeId);
    options.selectEdge(null);
  }

  return {
    createNodeAt,
    deleteNode,
    disconnectNodeEdges,
    duplicateNode,
    editNode,
  };
}
