import type { LogEntry, PipelineView } from '../lib/pipeline-types';

export function diffPipelineViews(
  prev: PipelineView | null | undefined,
  curr: PipelineView | null | undefined,
): LogEntry[] {
  if (!prev?.nodes || !curr?.nodes) return [];

  const prevMap = new Map(prev.nodes.map((node) => [node.nodeId, node.status]));
  const entries: LogEntry[] = [];

  for (const node of curr.nodes) {
    const prevStatus = prevMap.get(node.nodeId);
    if (prevStatus != null && prevStatus !== node.status) {
      entries.push({
        nodeId: node.nodeId,
        from: prevStatus,
        to: node.status,
        timestamp: Date.now(),
      });
    }
  }

  return entries;
}
