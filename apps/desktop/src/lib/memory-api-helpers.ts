// Shared low-level helpers + read/graph response normalizers for memory API.
// Extracted from useMemoryApi.ts to keep modules under 300 LOC.
import type {
  BuildContextResult,
  MemoryFile,
  SimilarConceptResult,
} from './workflow-types';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

export function toNonEmptyString(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : null;
}

export function extractUri(value: unknown): string | null {
  if (typeof value === 'string') {
    const direct = value.match(/memory:\/\/[^\s)]+/i);
    return direct?.[0] ?? null;
  }

  if (!isRecord(value)) return null;

  for (const key of ['uri', 'memoryUri', 'identifier', 'path']) {
    const candidate = toNonEmptyString(value[key]);
    if (candidate) return candidate;
  }

  return null;
}

export function extractLineageValue(
  record: Record<string, unknown>,
  ...keys: string[]
): string | null {
  for (const key of keys) {
    const value = toNonEmptyString(record[key]);
    if (value) return value;
  }
  return null;
}

export function normalizeReadMemoryResponse(value: unknown): MemoryFile {
  const data = unwrapToolData<unknown>(value);
  if (!isRecord(data)) {
    throw new Error('Memory read returned an unexpected response.');
  }

  const uri = extractUri(data);
  const content = typeof data.content === 'string' ? data.content : null;
  if (!uri || content === null) {
    throw new Error('Memory read did not include the expected content.');
  }

  const metadata = isRecord(data.metadata)
    ? data.metadata
    : isRecord(data.frontmatter)
      ? data.frontmatter
      : null;

  const metadataRecord = metadata ?? {};

  return {
    uri,
    title: toNonEmptyString(data.title) ?? uri,
    content,
    checksum: toNonEmptyString(data.checksum),
    created: typeof data.created === 'string' ? data.created : null,
    modified: typeof data.modified === 'string' ? data.modified : null,
    lineageGateTaskId: extractLineageValue(data, 'lineageGateTaskId', 'lineage_gate_task_id')
      ?? extractLineageValue(metadataRecord, 'lineageGateTaskId', 'lineage_gate_task_id'),
    lineagePipelineRunId: extractLineageValue(data, 'lineagePipelineRunId', 'lineage_pipeline_run_id')
      ?? extractLineageValue(metadataRecord, 'lineagePipelineRunId', 'lineage_pipeline_run_id'),
    lineageThinkingSessionId: extractLineageValue(data, 'lineageThinkingSessionId', 'lineage_thinking_session_id')
      ?? extractLineageValue(metadataRecord, 'lineageThinkingSessionId', 'lineage_thinking_session_id'),
    lineageMemoryNoteId: extractLineageValue(data, 'lineageMemoryNoteId', 'lineage_memory_note_id')
      ?? extractLineageValue(metadataRecord, 'lineageMemoryNoteId', 'lineage_memory_note_id'),
    parentWorkflowId: extractLineageValue(data, 'parentWorkflowId', 'parent_workflow_id')
      ?? extractLineageValue(metadataRecord, 'parentWorkflowId', 'parent_workflow_id'),
    ...(metadata ? { metadata } : {}),
  };
}

export function normalizeMemoryWriteResponse(value: unknown, fallbackUri: string | null = null): string {
  const data = unwrapToolData<unknown>(value);
  const uri = extractUri(data) ?? fallbackUri;
  if (!uri) {
    throw new Error('Memory write completed without returning a memory URI.');
  }
  return uri;
}

export function normalizeBuildContextResponse(value: unknown): BuildContextResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const directRelations = Array.isArray(record.DirectRelations)
    ? record.DirectRelations
    : Array.isArray(record.directRelations)
      ? record.directRelations
      : [];
  const expandedRelations = Array.isArray(record.ExpandedRelations)
    ? record.ExpandedRelations
    : Array.isArray(record.expandedRelations)
      ? record.expandedRelations
      : [];

  const normalizeRelation = (entry: unknown) => {
    if (!isRecord(entry)) return null;

    const name = toNonEmptyString(entry.Name) ?? toNonEmptyString(entry.name);
    if (!name) return null;

    const files = Array.isArray(entry.Files)
      ? entry.Files
      : Array.isArray(entry.files)
        ? entry.files
        : [];

    return {
      name,
      coOccurrenceCount:
        typeof entry.CoOccurrenceCount === 'number'
          ? entry.CoOccurrenceCount
          : typeof entry.coOccurrenceCount === 'number'
            ? entry.coOccurrenceCount
            : 0,
      files: files.flatMap((file) => (typeof file === 'string' ? [file] : [])),
      weightedScore:
        typeof entry.WeightedScore === 'number'
          ? entry.WeightedScore
          : typeof entry.weightedScore === 'number'
            ? entry.weightedScore
            : null,
      communityId:
        typeof entry.CommunityId === 'number'
          ? entry.CommunityId
          : typeof entry.communityId === 'number'
            ? entry.communityId
            : null,
    };
  };

  const conceptName = toNonEmptyString(record.ConceptName) ?? toNonEmptyString(record.conceptName) ?? '';
  if (!conceptName) {
    throw new Error('BuildContext did not include a concept name.');
  }

  return {
    conceptName,
    depth:
      typeof record.Depth === 'number'
        ? record.Depth
        : typeof record.depth === 'number'
          ? record.depth
          : 0,
    communityId:
      typeof record.CommunityId === 'number'
        ? record.CommunityId
        : typeof record.communityId === 'number'
          ? record.communityId
          : null,
    directRelations: directRelations.flatMap((entry) => {
      const relation = normalizeRelation(entry);
      return relation ? [relation] : [];
    }),
    expandedRelations: expandedRelations.flatMap((entry) => {
      const relation = normalizeRelation(entry);
      return relation ? [relation] : [];
    }),
  };
}

export function normalizeSimilarConceptsResponse(value: unknown): SimilarConceptResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const conceptName = toNonEmptyString(record.conceptName) ?? toNonEmptyString(record.ConceptName) ?? '';
  if (!conceptName) {
    throw new Error('FindSimilarConcepts did not include a concept name.');
  }

  const collection = Array.isArray(record.results)
    ? record.results
    : Array.isArray(record.Results)
      ? record.Results
      : [];

  return {
    conceptName,
    totalCount:
      typeof record.totalCount === 'number'
        ? record.totalCount
        : typeof record.TotalCount === 'number'
          ? record.TotalCount
          : collection.length,
    results: collection.flatMap((entry) => {
      if (!isRecord(entry)) return [];
      const name = toNonEmptyString(entry.name) ?? toNonEmptyString(entry.Name);
      if (!name) return [];
      return [{
        name,
        similarity:
          typeof entry.similarity === 'number'
            ? entry.similarity
            : typeof entry.Similarity === 'number'
              ? entry.Similarity
              : 0,
        weightedSimilarity:
          typeof entry.weightedSimilarity === 'number'
            ? entry.weightedSimilarity
            : typeof entry.WeightedSimilarity === 'number'
              ? entry.WeightedSimilarity
              : null,
      }];
    }),
  };
}
