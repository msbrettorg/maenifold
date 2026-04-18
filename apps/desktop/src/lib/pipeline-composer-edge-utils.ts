import { getSmoothStepPath, Position, type EdgeProps } from '@vue-flow/core';

import type { FlowNodeData } from './pipeline-composer-flow';

export type FlowEdgeData = {
  edgeType: 'success' | 'always';
  mapping?: Record<string, string>;
  /** Absolute bend points from ELK layout. Present only after auto-layout; cleared on manual drag. */
  elkPoints?: Array<{ x: number; y: number }>;
  /** 0-based position of this edge among its source's outgoing edges. */
  siblingIndex?: number;
  /** Total number of outgoing edges sharing this edge's source node. */
  siblingTotal?: number;
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

/**
 * Build an SVG path from ELK's absolute routing points with rounded orthogonal
 * corners. Uses quadratic Bézier curves at each bend point so edges match the
 * `borderRadius: 12` aesthetic of the smoothstep fallback.
 */
function elkPointsToSvgPath(
  points: Array<{ x: number; y: number }>,
  radius = 12,
): string {
  if (points.length < 2) return '';

  const parts: string[] = [`M ${points[0].x} ${points[0].y}`];

  for (let i = 1; i < points.length - 1; i++) {
    const prev = points[i - 1];
    const curr = points[i];
    const next = points[i + 1];

    const dx1 = curr.x - prev.x;
    const dy1 = curr.y - prev.y;
    const dx2 = next.x - curr.x;
    const dy2 = next.y - curr.y;

    const len1 = Math.hypot(dx1, dy1);
    const len2 = Math.hypot(dx2, dy2);

    if (len1 === 0 || len2 === 0) {
      parts.push(`L ${curr.x} ${curr.y}`);
      continue;
    }

    const r = Math.min(radius, len1 / 2, len2 / 2);

    const x1 = curr.x - (dx1 / len1) * r;
    const y1 = curr.y - (dy1 / len1) * r;
    const x2 = curr.x + (dx2 / len2) * r;
    const y2 = curr.y + (dy2 / len2) * r;

    parts.push(`L ${x1} ${y1}`);
    parts.push(`Q ${curr.x} ${curr.y} ${x2} ${y2}`);
  }

  const last = points[points.length - 1];
  parts.push(`L ${last.x} ${last.y}`);

  return parts.join(' ');
}

/** Compute the midpoint along a polyline defined by `points` for label placement. */
function elkPointsMidpoint(points: Array<{ x: number; y: number }>): { x: number; y: number } {
  if (points.length === 0) return { x: 0, y: 0 };
  if (points.length === 1) return { x: points[0].x, y: points[0].y };

  let totalLength = 0;
  const segmentLengths: number[] = [];
  for (let i = 1; i < points.length; i++) {
    const len = Math.hypot(points[i].x - points[i - 1].x, points[i].y - points[i - 1].y);
    segmentLengths.push(len);
    totalLength += len;
  }

  let remaining = totalLength / 2;
  for (let i = 0; i < segmentLengths.length; i++) {
    if (remaining <= segmentLengths[i]) {
      const t = segmentLengths[i] > 0 ? remaining / segmentLengths[i] : 0;
      return {
        x: points[i].x + (points[i + 1].x - points[i].x) * t,
        y: points[i].y + (points[i + 1].y - points[i].y) * t,
      };
    }
    remaining -= segmentLengths[i];
  }

  return { x: points[points.length - 1].x, y: points[points.length - 1].y };
}

/** Pixel spacing between sibling fan-out lanes for the smoothstep fallback. */
const SIBLING_LANE_SPACING = 28;

/**
 * Compute a per-edge offset that shifts the smoothstep "bus" segment so
 * sibling edges sharing a source node don't collapse onto a single
 * horizontal/vertical lane. Returns 0 when the edge has no recorded siblings
 * or is the only outgoing edge from its source.
 */
function siblingLaneOffset(data?: FlowEdgeData): number {
  const total = data?.siblingTotal ?? 0;
  const index = data?.siblingIndex ?? 0;
  if (total <= 1) return 0;
  return (index - (total - 1) / 2) * SIBLING_LANE_SPACING;
}

export function edgePath(edge: EdgeProps<FlowEdgeData>): { d: string; x: number; y: number } {
  const points = edge.data?.elkPoints;
  if (points && points.length >= 2) {
    const d = elkPointsToSvgPath(points);
    const mid = elkPointsMidpoint(points);
    return { d, x: mid.x, y: mid.y };
  }

  const lane = siblingLaneOffset(edge.data);
  const verticalFlow =
    edge.sourcePosition === Position.Top || edge.sourcePosition === Position.Bottom;
  const centerX = verticalFlow ? undefined : (edge.sourceX + edge.targetX) / 2 + lane;
  const centerY = verticalFlow ? (edge.sourceY + edge.targetY) / 2 + lane : undefined;

  const [d, x, y] = getSmoothStepPath({
    sourceX: edge.sourceX,
    sourceY: edge.sourceY,
    sourcePosition: edge.sourcePosition,
    targetX: edge.targetX,
    targetY: edge.targetY,
    targetPosition: edge.targetPosition,
    offset: 20,
    borderRadius: 12,
    centerX,
    centerY,
  });
  return { d, x, y };
}
