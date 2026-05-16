import type { Edge, Node } from '@vue-flow/core';

import type {
  EdgeCondition,
  PipelineDefinition,
  PipelineEdge,
} from './pipeline-types-edge';
import { toPipelineNode, type FlowNodeData } from './pipeline-composer-flow-nodes';

export function pipelineEdgesToFlow(pipelineEdges: PipelineEdge[]): Edge[] {
  const normalizedCondition = (edge: PipelineEdge): EdgeCondition | undefined => {
    if (edge.condition) {
      return edge.condition;
    }
    if (
      typeof edge.path === 'string' &&
      edge.path.length > 0 &&
      (typeof edge.equals === 'string' || typeof edge.equals === 'number' || typeof edge.equals === 'boolean')
    ) {
      return { path: edge.path, equals: edge.equals };
    }
    return undefined;
  };

  const edgeLabel = (edge: PipelineEdge): string => {
    const condition = normalizedCondition(edge);
    return edge.type === 'conditional' && condition
      ? `?{${condition.path}}=${JSON.stringify(condition.equals)}`
      : edge.type ?? 'success';
  };

  return pipelineEdges.map((edge) => ({
    id: `${edge.source}->${edge.target}`,
    source: edge.source,
    target: edge.target,
    type: 'smoothstep',
    animated: edge.type === 'always',
    label: edgeLabel(edge),
    data: {
      edgeType: edge.type ?? 'success',
      ...(() => {
        const condition = normalizedCondition(edge);
        return condition ? { condition: { ...condition } } : {};
      })(),
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
    edges: flowEdges.map((edge) => {
      const edgeType = (edge.data?.edgeType as 'success' | 'always' | 'conditional') ?? 'success';
      const condition = edge.data?.condition as Partial<EdgeCondition> | undefined;
      const path = typeof condition?.path === 'string' ? condition.path : '';
      const equals = condition?.equals;
      const hasValidCondition =
        path.length > 0 &&
        (typeof equals === 'string' || typeof equals === 'number' || typeof equals === 'boolean');

      return {
        source: edge.source,
        target: edge.target,
        type: edgeType,
        ...(edgeType === 'conditional' && hasValidCondition ? { path, equals } : {}),
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
      };
    }),
  };
}
