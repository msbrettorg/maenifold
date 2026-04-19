import { getSmoothStepPath, type EdgeProps } from '@vue-flow/core';

import type { EdgeConditionEquals } from './pipeline-types';

type FlowEdgeCondition = {
  path: string;
  equals: EdgeConditionEquals;
};

export type FlowEdgeData = {
  edgeType: 'success' | 'always' | 'conditional';
  condition?: FlowEdgeCondition;
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
  return null;
}

function truncate(text: string, max = 20): string {
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}

function conditionalLabel(condition: FlowEdgeCondition): string {
  const raw = `?{${condition.path}}=${JSON.stringify(condition.equals)}`;
  return truncate(raw);
}

function conditionalLabelFull(condition: FlowEdgeCondition): string {
  return `?{${condition.path}}=${JSON.stringify(condition.equals)}`;
}

export function edgeLabel(data?: FlowEdgeData): string | null {
  if (data?.edgeType !== 'conditional' || !data.condition?.path) return null;
  return conditionalLabel(data.condition);
}

export function edgeLabelTitle(data?: FlowEdgeData): string | null {
  if (data?.edgeType !== 'conditional' || !data.condition?.path) return null;
  return conditionalLabelFull(data.condition);
}

export function edgePathStyle(
  edge: EdgeProps<FlowEdgeData>,
): Record<string, string | number> | undefined {
  const edgeType = edge.data?.edgeType ?? 'success';
  if (edgeType === 'always') {
    return { ...(edge.style ?? {}), strokeDasharray: '8 6' };
  }
  if (edgeType === 'conditional') {
    return { ...(edge.style ?? {}), strokeDasharray: '2 6' };
  }
  return edge.style as Record<string, string | number> | undefined;
}

export function edgePath(edge: EdgeProps<FlowEdgeData>): { d: string; x: number; y: number } {
  const [d, x, y] = getSmoothStepPath({
    sourceX: edge.sourceX,
    sourceY: edge.sourceY,
    sourcePosition: edge.sourcePosition,
    targetX: edge.targetX,
    targetY: edge.targetY,
    targetPosition: edge.targetPosition,
    offset: 20,
    borderRadius: 12,
  });
  return { d, x, y };
}
