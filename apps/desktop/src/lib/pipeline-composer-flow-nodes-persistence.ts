import type { Node } from '@vue-flow/core';

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
