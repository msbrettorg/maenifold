// Search/recent normalizers for memory API. Extracted from useMemoryApi.ts
// to keep modules under 300 LOC.
import type { RecentResult, SearchResult } from './workflow-types';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import { extractUri, isRecord, toNonEmptyString } from './memory-api-helpers';

function normalizeSearchItem(value: unknown): SearchResult['results'][number] | null {
  if (!isRecord(value)) return null;

  const uri = extractUri(value);
  if (!uri) return null;

  return {
    uri,
    title: toNonEmptyString(value.title) ?? uri,
    snippet: toNonEmptyString(value.snippet) ?? toNonEmptyString(value.preview) ?? '',
    score:
      typeof value.score === 'number'
        ? value.score
        : typeof value.rank === 'number'
          ? value.rank
          : 0,
  };
}

export function normalizeSearchResponse(value: unknown, fallbackPageSize: number): SearchResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const collection = Array.isArray(record.results)
    ? record.results
    : Array.isArray(record.items)
      ? record.items
      : Array.isArray(data)
        ? data
        : [];

  const results = collection
    .map((entry) => normalizeSearchItem(entry))
    .filter((entry): entry is SearchResult['results'][number] => entry !== null)
    .sort((left, right) => right.score - left.score);

  return {
    results,
    totalCount: typeof record.totalCount === 'number' ? record.totalCount : results.length,
    page: typeof record.page === 'number' ? record.page : 1,
    pageSize: typeof record.pageSize === 'number' ? record.pageSize : fallbackPageSize,
  };
}

function normalizeRecentItem(value: unknown): RecentResult['items'][number] | null {
  if (!isRecord(value)) return null;

  const uri = extractUri(value);
  const modified =
    toNonEmptyString(value.modified)
    ?? toNonEmptyString(value.updatedAt)
    ?? toNonEmptyString(value.created);

  if (!uri || !modified) return null;

  const rawSections = Array.isArray(value.sections)
    ? value.sections
    : Array.isArray(value.headings)
      ? value.headings
      : [];

  return {
    uri,
    title: toNonEmptyString(value.title) ?? uri,
    modified,
    sections: rawSections.flatMap((section) => {
      if (typeof section === 'string') return [section];
      if (isRecord(section)) {
        const title = toNonEmptyString(section.title) ?? toNonEmptyString(section.heading);
        return title ? [title] : [];
      }
      return [];
    }),
  };
}

export function normalizeRecentResponse(value: unknown): RecentResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const collection = Array.isArray(record.items)
    ? record.items
    : Array.isArray(record.results)
      ? record.results
      : Array.isArray(data)
        ? data
        : [];

  return {
    items: collection
      .map((entry) => normalizeRecentItem(entry))
      .filter((entry): entry is RecentResult['items'][number] => entry !== null),
  };
}
