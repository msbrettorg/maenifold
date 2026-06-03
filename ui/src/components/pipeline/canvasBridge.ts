// Flow-bridge: pure store-shape ⇄ vue-flow-shape mapping for the Pipelines canvas.
//
// This module owns the single conversion boundary between the `pipelines` store
// (PipelineView / ViewNode / PipelineEdge) and the Vue Flow runtime (Node / Edge).
// It is intentionally free of Vue reactivity and of any component import so the
// mapping can be reasoned about (and unit-tested) in isolation. The canvas
// composables (useCanvasSync / useCanvasLayout) and the custom node/edge
// components are the only consumers.

import { getSmoothStepPath, Position } from '@vue-flow/core';
import type { Edge, Node } from '@vue-flow/core';
import type {
  EdgeConditionEquals,
  EdgeMode,
  EdgeType,
  NodeStatus,
  PipelineEdge,
  PipelineNodeType,
  PipelineView,
  ViewNode,
} from '../../lib/pipeline-types';
import {
  isPromptViewNode,
  isSequentialThinkingViewNode,
  isToolViewNode,
  viewNodeDisplayLabel,
  viewNodeType,
} from '../../lib/pipeline-types';

// --- geometry / layout constants -------------------------------------------

export const NODE_W = 264;
export const NODE_H = 132;

/** Pixel separation between adjacent edges that share one source handle. */
const FAN_SHELF_SPACING = 18;

export type Direction = 'LR' | 'TB';

export const DEFAULT_DIRECTION: Direction = 'LR';

// LR draws left→right (source handle on the right), TB draws top→bottom.
export const DIRECTION_HANDLES: Record<Direction, { source: string; target: string }> = {
  LR: { source: 'out-right', target: 'in-left' },
  TB: { source: 'out-bottom', target: 'in-top' },
};

export type XY = { x: number; y: number };

export function fallbackGrid(index: number): XY {
  return { x: (index % 4) * 320 + 40, y: Math.floor(index / 4) * 220 + 40 };
}

// --- status normalization ---------------------------------------------------

export type StatusKey = 'pending' | 'active' | 'stuck' | 'completed' | 'abandoned';

export function statusKey(status?: NodeStatus): StatusKey {
  switch (status) {
    case 'active':
    case 'retrying':
      return 'active';
    case 'completed':
      return 'completed';
    case 'abandoned':
    case 'cancelled':
      return 'abandoned';
    case 'stuck':
    case 'failed':
      return 'stuck';
    case 'pending':
    case 'ready':
    case 'skipped':
    default:
      return 'pending';
  }
}

// --- node data carried on each Vue Flow node --------------------------------

export interface FlowNodeData {
  nodeId: string;
  label: string;
  nodeType: PipelineNodeType;
  status: NodeStatus;
  statusKey: StatusKey;
  attempts: number;
  subtitle: string;
  detail: string;
  // Tool nodes execute deterministically with zero LLM tokens — the product's
  // flagship affordance. Every tool node earns the celebratory "0 tokens" badge.
  zeroTokens: boolean;
  sessionBound: boolean;
}

function truncate(text: string, max: number): string {
  const trimmed = text.trim();
  return trimmed.length > max ? `${trimmed.slice(0, max - 1)}…` : trimmed;
}

function nodeSubtitleDetail(node: ViewNode): { subtitle: string; detail: string } {
  if (isToolViewNode(node)) {
    return { subtitle: node.toolName, detail: '' };
  }
  if (isPromptViewNode(node)) {
    return { subtitle: node.agent, detail: truncate(node.prompt ?? '', 96) };
  }
  if (isSequentialThinkingViewNode(node)) {
    return { subtitle: node.agent, detail: `topic: ${truncate(node.topic ?? '', 72)}` };
  }
  // workflow (default)
  const workflowId = 'workflowId' in node ? String(node.workflowId ?? '') : '';
  const prompt = 'prompt' in node && node.prompt ? truncate(node.prompt, 96) : '';
  return { subtitle: workflowId ? `workflow: ${workflowId}` : 'workflow', detail: prompt };
}

function toFlowNodeData(node: ViewNode): FlowNodeData {
  const nodeType = viewNodeType(node);
  const { subtitle, detail } = nodeSubtitleDetail(node);
  const sessionBound = isPromptViewNode(node) && Boolean(node.sessionRef?.sessionId);
  return {
    nodeId: node.nodeId,
    label: viewNodeDisplayLabel(node),
    nodeType,
    status: node.status,
    statusKey: statusKey(node.status),
    attempts: typeof node.attempts === 'number' ? node.attempts : 0,
    subtitle,
    detail,
    zeroTokens: nodeType === 'tool',
    sessionBound,
  };
}

export function viewToFlowNodes(view: PipelineView, cached: Record<string, XY>): Node<FlowNodeData>[] {
  return view.nodes.map((node, index) => ({
    id: node.nodeId,
    type: 'pipeline',
    position: cached[node.nodeId] ?? fallbackGrid(index),
    data: toFlowNodeData(node),
  }));
}

// --- edge data + mapping ----------------------------------------------------

export interface FlowEdgeData {
  edgeType: EdgeType;
  path?: string;
  equals?: EdgeConditionEquals;
  mode?: EdgeMode;
  mapping?: Record<string, string>;
  /** 0-based index of this edge among siblings sharing one source handle. */
  fanIndex: number;
  /** Total edges sharing this edge's source handle. */
  fanCount: number;
}

function sanitizeMapping(mapping?: Record<string, string> | null): Record<string, string> | undefined {
  if (!mapping || typeof mapping !== 'object') return undefined;
  const entries = Object.entries(mapping).filter(
    (entry): entry is [string, string] =>
      typeof entry[0] === 'string' && typeof entry[1] === 'string'
  );
  return entries.length > 0 ? Object.fromEntries(entries) : {};
}

export function viewToFlowEdges(edges: PipelineEdge[], direction: Direction): Edge<FlowEdgeData>[] {
  const handles = DIRECTION_HANDLES[direction] ?? DIRECTION_HANDLES.LR;
  const sourceCount = new Map<string, number>();
  for (const edge of edges) {
    sourceCount.set(edge.source, (sourceCount.get(edge.source) ?? 0) + 1);
  }
  const sourceIndex = new Map<string, number>();
  return edges.map((edge) => {
    const idx = sourceIndex.get(edge.source) ?? 0;
    sourceIndex.set(edge.source, idx + 1);
    const count = sourceCount.get(edge.source) ?? 1;
    // The store guarantees `type` is present (the gateway rejects typeless
    // edges); default defensively only so a malformed payload still renders.
    const edgeType: EdgeType = edge.type ?? 'success';
    const data: FlowEdgeData = {
      edgeType,
      ...(typeof edge.path === 'string' ? { path: edge.path } : {}),
      ...(edge.equals !== undefined ? { equals: edge.equals } : {}),
      ...(edge.mode ? { mode: edge.mode } : {}),
      ...(edge.mapping ? { mapping: sanitizeMapping(edge.mapping) } : {}),
      fanIndex: idx,
      fanCount: count,
    };
    return {
      id: `${edge.source}->${edge.target}#${idx}`,
      source: edge.source,
      target: edge.target,
      sourceHandle: handles.source,
      targetHandle: handles.target,
      type: 'pipeline',
      data,
    };
  });
}

// --- edge visual helpers (consumed by PipelineEdge.vue) ---------------------

export function edgeDashArray(edgeType: EdgeType): string | undefined {
  if (edgeType === 'always') return '8 6';
  if (edgeType === 'conditional') return '2 6';
  return undefined; // success = solid
}

export function edgeConditionLabel(data?: FlowEdgeData): string | null {
  if (!data || data.edgeType !== 'conditional' || !data.path) return null;
  return `?{${data.path}}=${JSON.stringify(data.equals)}`;
}

export type MappingIndicator = 'ok' | 'warn';

export function edgeMappingIndicator(data?: FlowEdgeData): MappingIndicator | null {
  if (!data?.mapping) return null;
  // A declared-but-empty mapping (or a binding-mode edge with no field map) is
  // a misconfiguration the operator should see: warn. A populated map is ok.
  if (Object.keys(data.mapping).length === 0) return 'warn';
  return 'ok';
}

export function edgeFanOffset(data?: FlowEdgeData): number {
  const idx = data?.fanIndex ?? 0;
  const count = data?.fanCount ?? 1;
  return count > 1 ? (idx - (count - 1) / 2) * FAN_SHELF_SPACING : 0;
}

export interface EdgeGeometryInput {
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  sourcePosition: Position;
  targetPosition: Position;
  data?: FlowEdgeData;
}

export function buildEdgePath(input: EdgeGeometryInput): { d: string; labelX: number; labelY: number } {
  const fanOffset = edgeFanOffset(input.data);
  const isVertical =
    input.sourcePosition === Position.Top || input.sourcePosition === Position.Bottom;
  const midX = (input.sourceX + input.targetX) / 2;
  const midY = (input.sourceY + input.targetY) / 2;
  const [d, labelX, labelY] = getSmoothStepPath({
    sourceX: input.sourceX,
    sourceY: input.sourceY,
    sourcePosition: input.sourcePosition,
    targetX: input.targetX,
    targetY: input.targetY,
    targetPosition: input.targetPosition,
    offset: 20,
    borderRadius: 12,
    centerX: isVertical ? undefined : midX + fanOffset,
    centerY: isVertical ? midY + fanOffset : undefined,
  });
  return { d, labelX, labelY };
}

// --- per-pipeline persistence (localStorage) --------------------------------

function positionCacheKey(pipelineId?: string | null): string {
  return `maenifold:pipeline:positions:v1:${pipelineId ?? 'design'}`;
}

function directionKey(pipelineId: string): string {
  return `maenifold:pipeline:direction:v1:${pipelineId}`;
}

export function loadPositions(pipelineId?: string | null): Record<string, XY> {
  try {
    const raw = globalThis.localStorage?.getItem(positionCacheKey(pipelineId));
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== 'object' || parsed === null) return {};
    const out: Record<string, XY> = {};
    for (const [id, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (
        typeof value === 'object' &&
        value !== null &&
        typeof (value as XY).x === 'number' &&
        typeof (value as XY).y === 'number'
      ) {
        out[id] = { x: (value as XY).x, y: (value as XY).y };
      }
    }
    return out;
  } catch {
    return {};
  }
}

export function savePositions(
  nodes: Array<{ id: string; position: XY }>,
  pipelineId?: string | null
): void {
  const positions: Record<string, XY> = {};
  for (const node of nodes) {
    positions[node.id] = { x: node.position.x, y: node.position.y };
  }
  try {
    globalThis.localStorage?.setItem(positionCacheKey(pipelineId), JSON.stringify(positions));
  } catch {
    // localStorage may be unavailable (private mode / quota) — non-fatal.
  }
}

export function loadDirection(pipelineId: string): Direction {
  try {
    const raw = globalThis.localStorage?.getItem(directionKey(pipelineId));
    return raw === 'TB' ? 'TB' : DEFAULT_DIRECTION;
  } catch {
    return DEFAULT_DIRECTION;
  }
}

export function saveDirection(pipelineId: string, direction: Direction): void {
  try {
    globalThis.localStorage?.setItem(directionKey(pipelineId), direction);
  } catch {
    // non-fatal
  }
}
