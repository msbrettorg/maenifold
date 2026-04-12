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
  return view.nodes.flatMap((node, index) => {
    const loadedAt = Date.now();
    const entries: LogEntry[] = [];
    if (node.status === 'pending') {
      return entries;
    }
    if (node.status === 'completed') {
      const preview = buildNodeOutputPreview(node);
      entries.push({
        nodeId: node.nodeId,
        from: 'loaded',
        to: node.status,
        timestamp: loadedAt + index,
        ...(preview ? { preview } : {}),
      });
    } else if (node.status === 'abandoned' || node.status === 'stuck') {
      const preview = buildNodeOutputPreview(node);
      entries.push({
        nodeId: node.nodeId,
        from: 'loaded',
        to: node.status,
        timestamp: loadedAt + index,
        ...(preview ? { preview } : {}),
      });
    } else if (node.status === 'retrying') {
      entries.push({
        nodeId: node.nodeId,
        from: 'loaded',
        to: node.status,
        timestamp: loadedAt + index,
        preview: `Retrying${node.output ? ': ' + node.output.slice(0, 80) : ''}`,
      });
    } else {
      entries.push({
        nodeId: node.nodeId,
        from: 'loaded',
        to: node.status,
        timestamp: loadedAt + index,
      });
    }
    return entries;
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
      const preview =
        (node.status === 'abandoned' || node.status === 'stuck')
          ? buildNodeOutputPreview(node)
          : '';
      entries.push({
        nodeId: node.nodeId,
        from: prevStatus,
        to: node.status,
        timestamp: Date.now(),
        ...(preview ? { preview } : {}),
      });
    }
  }

  return entries;
}
