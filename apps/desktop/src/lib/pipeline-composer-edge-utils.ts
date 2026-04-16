import { getSmoothStepPath, type EdgeProps } from '@vue-flow/core';

import type { FlowNodeData } from './pipeline-composer-flow';

export type FlowEdgeData = {
  edgeType: 'success' | 'always';
  mapping?: Record<string, string>;
};

function edgeMappings(data?: FlowEdgeData): Array<[string, string]> {
  return Object.entries(data?.mapping ?? {}).filter(
    (entry): entry is [string, string] =>
      typeof entry[0] === 'string' && typeof entry[1] === 'string',
  );
}

export function edgeTooltip(data?: FlowEdgeData): string {
  const mappings = edgeMappings(data);
  return mappings.length > 0
    ? mappings.map(([targetField, sourceField]) => `${sourceField} → ${targetField}`).join('\n')
    : 'No mapping configured';
}

export function edgeIndicator(edge: EdgeProps<FlowEdgeData>): 'mapped' | 'warning' | null {
  if (edgeMappings(edge.data).length > 0) return 'mapped';
  return (edge.targetNode.data as FlowNodeData | undefined)?.type === 'tool'
    ? 'warning'
    : null;
}

export function edgePath(edge: EdgeProps<FlowEdgeData>): { d: string; x: number; y: number } {
  const [d, x, y] = getSmoothStepPath({ ...edge, offset: 20, borderRadius: 12 });
  return { d, x, y };
}
