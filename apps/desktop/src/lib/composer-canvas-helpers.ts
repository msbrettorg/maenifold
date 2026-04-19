// Pure helpers and types for ComposerCanvas. Extracted from
// composables/useComposerCanvas.ts to keep modules under 300 LOC.
import type { Edge } from '@vue-flow/core';
import type { PipelineView } from './pipeline-types';

export type Point = { x: number; y: number };
export type SelectableFlowElement = { selected?: boolean };
export type Direction = 'LR' | 'TB';

export const DEFAULT_DIRECTION: Direction = 'LR';

export const DIRECTION_HANDLES: Record<Direction, { source: string; target: string }> = {
  LR: { source: 'out-right', target: 'in-left' },
  TB: { source: 'out-bottom', target: 'in-top' },
};

export function directionStorageKey(sessionId: string): string {
  return `maenifold:pipeline:${sessionId}:direction`;
}

export function parseDirection(raw: string | null): Direction {
  return raw === 'TB' ? 'TB' : DEFAULT_DIRECTION;
}

export function loadDirectionPreference(sessionId: string): Direction {
  try {
    return parseDirection(globalThis.localStorage?.getItem(directionStorageKey(sessionId)) ?? null);
  } catch {
    return DEFAULT_DIRECTION;
  }
}

export function saveDirectionPreference(sessionId: string, direction: Direction): void {
  try {
    globalThis.localStorage?.setItem(directionStorageKey(sessionId), direction);
  } catch {
    // Ignore localStorage failures.
  }
}

export function isSelected(element: SelectableFlowElement): boolean {
  return !!element.selected;
}

export function applyDirectionHandles(edgeList: Edge[], direction: Direction): Edge[] {
  const handles = DIRECTION_HANDLES[direction] ?? DIRECTION_HANDLES.LR;
  const sourceCount = new Map<string, number>();
  for (const edge of edgeList) {
    sourceCount.set(edge.source, (sourceCount.get(edge.source) ?? 0) + 1);
  }
  const sourceIndex = new Map<string, number>();
  return edgeList.map((edge) => {
    const idx = sourceIndex.get(edge.source) ?? 0;
    sourceIndex.set(edge.source, idx + 1);
    const count = sourceCount.get(edge.source) ?? 1;
    return {
      ...edge,
      sourceHandle: handles.source,
      targetHandle: handles.target,
      data: {
        ...(edge.data ?? {}),
        fanIndex: idx,
        fanCount: count,
      },
    };
  });
}

export function hasViewStructureChanged(
  view: PipelineView | null | undefined,
  previousView: PipelineView | null | undefined,
): boolean {
  if (!view && !previousView) return false;
  if (!view || !previousView) return true;

  const nodeIds = new Set(view.nodes.map((node) => node.nodeId));
  const previousNodeIds = new Set(previousView.nodes.map((node) => node.nodeId));
  if (nodeIds.size !== previousNodeIds.size) return true;
  for (const nodeId of nodeIds) {
    if (!previousNodeIds.has(nodeId)) return true;
  }

  const edgePairs = new Set(view.edges.map((edge) => `${edge.source}->${edge.target}`));
  const previousEdgePairs = new Set(
    previousView.edges.map((edge) => `${edge.source}->${edge.target}`),
  );
  if (edgePairs.size !== previousEdgePairs.size) return true;
  for (const pair of edgePairs) {
    if (!previousEdgePairs.has(pair)) return true;
  }

  return false;
}

export function toFlowPosition(
  clientX: number,
  clientY: number,
  rootElement: HTMLElement,
): Point {
  const rootRect = rootElement.getBoundingClientRect();
  const viewportElement = rootElement.querySelector<HTMLElement>('.vue-flow__viewport');
  const transform = viewportElement ? getComputedStyle(viewportElement).transform : 'none';
  const matrix = transform && transform !== 'none'
    ? new DOMMatrixReadOnly(transform)
    : new DOMMatrixReadOnly();
  const zoom = matrix.a || 1;

  return {
    x: Math.round(((clientX - rootRect.left - matrix.m41) / zoom) / 20) * 20,
    y: Math.round(((clientY - rootRect.top - matrix.m42) / zoom) / 20) * 20,
  };
}
