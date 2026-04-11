import type { LogEntry, PipelineView } from '../lib/pipeline-types';

const OUTPUT_PREVIEW_LENGTH = 100;

export function buildNodeOutputPreview(
  node: { output?: string } | null | undefined,
): string {
  const output = node?.output?.trim();
  if (!output) return '';
  const normalizedOutput = output.replace(/\s+/g, ' ');
  return normalizedOutput.length > OUTPUT_PREVIEW_LENGTH
    ? `${normalizedOutput.slice(0, OUTPUT_PREVIEW_LENGTH)}...`
    : normalizedOutput;
}

export function hydrateEventLogFromView(
  view: PipelineView | null | undefined,
): LogEntry[] {
  if (!view?.nodes) return [];
  const loadedAt = Date.now();
  return view.nodes.flatMap((node, index) => {
    if (node.status === 'pending') {
      return [];
    }
    const preview = node.status === 'completed' ? buildNodeOutputPreview(node) : '';
    return [{
      nodeId: node.nodeId,
      from: 'loaded',
      to: node.status,
      timestamp: loadedAt + index,
      ...(preview ? { preview } : {}),
    }];
  });
}

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
