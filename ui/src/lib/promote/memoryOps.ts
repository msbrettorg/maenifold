// Promote-to-memory backend bridge + pure result classification.
//
// The memory tools live in the `maenifold.memory` MCP domain. Reads/searches
// return structured JSON (unwrapped by `executeGateway`). Mutations (Write/Edit)
// can return a plain `ERROR: ...` string for validation/concurrency failures
// (e.g. the backend returns `Ok("ERROR: Checksum mismatch...")` on a stale edit),
// so we read their RAW text and classify it rather than trusting a thrown error.
//
// Backend op schema (crates/maenifold-core/src/memory/{access,editing}.rs +
// mcp/schema/memory_params.rs): EditMemory.operation ∈
// append | prepend | find_replace | replace_section. There is NO full-document
// replace — an "update" is therefore replace_section(sectionName) OR
// find_replace(expectedCount:1).

import { executeGateway, executeGatewayText } from '../mcp/gateway';
import type { MemoryFile, MemorySearchResult } from '../../stores/knowledge';

const MEMORY_DOMAIN = 'maenifold.memory';
const SEARCH_PAGE_SIZE = 12;

// ---------------------------------------------------------------------------
// Promote operation model
// ---------------------------------------------------------------------------

// 'create' → WriteMemory. The rest → EditMemory operations. 'update' surfaces in
// the UI as two concrete backend ops since there is no whole-file replace.
export type PromoteMode = 'create' | 'append' | 'prepend' | 'replace_section' | 'find_replace';

export interface EditMemoryRequest {
  identifier: string;
  operation: 'append' | 'prepend' | 'find_replace' | 'replace_section';
  content: string;
  checksum?: string;
  findText?: string;
  sectionName?: string;
  expectedCount?: number;
}

export interface WriteMemoryRequest {
  title: string;
  content: string;
  folder?: string;
}

// --- pure param builders (unit-tested) --------------------------------------

export function buildEditRequest(
  mode: Exclude<PromoteMode, 'create'>,
  identifier: string,
  content: string,
  checksum: string | undefined,
  extra: { findText?: string; sectionName?: string } = {}
): EditMemoryRequest {
  const base: EditMemoryRequest = { identifier, operation: mode, content, checksum };
  if (mode === 'replace_section') {
    base.sectionName = extra.sectionName?.trim() ?? '';
  }
  if (mode === 'find_replace') {
    base.findText = extra.findText ?? '';
    // Finding #6: an "update" via find_replace pins expectedCount to exactly 1 so
    // an ambiguous match (0 or many) fails loudly instead of silently editing.
    base.expectedCount = 1;
  }
  return base;
}

export function buildWriteRequest(title: string, content: string, folder?: string): WriteMemoryRequest {
  const trimmedFolder = folder?.trim();
  return {
    title: title.trim(),
    content,
    ...(trimmedFolder ? { folder: trimmedFolder } : {}),
  };
}

// ---------------------------------------------------------------------------
// Result classification (pure, unit-tested)
// ---------------------------------------------------------------------------

export type MemoryOutcomeKind = 'ok' | 'stale' | 'error';

export interface MemoryOutcome {
  kind: MemoryOutcomeKind;
  /** Present for `error`; the surfaced (backend-parity) message. */
  message?: string;
}

const CHECKSUM_MISMATCH = /checksum mismatch/i;
const ERROR_PREFIX = /(^|")\s*ERROR:\s*([^"\\]+)/i;
const VALIDATION_MARKERS = [
  'must contain at least one',
  'WikiLinks that are excluded',
  'VALIDATION_ERROR',
  'FILTERED_WIKILINKS',
];

/**
 * Classify the RAW text a memory mutation returned. A stale checksum is detected
 * first (it recovers to the editor, not a hard error). Then explicit `ERROR:`
 * strings and the known validation markers. Anything else is a success.
 */
export function classifyMemoryOutcome(rawText: string): MemoryOutcome {
  const text = rawText ?? '';
  if (CHECKSUM_MISMATCH.test(text)) return { kind: 'stale' };

  const errorMatch = ERROR_PREFIX.exec(text);
  if (errorMatch) return { kind: 'error', message: errorMatch[2].trim() };

  if (VALIDATION_MARKERS.some((marker) => text.includes(marker))) {
    return { kind: 'error', message: text.trim() };
  }
  return { kind: 'ok' };
}

/**
 * Classify a thrown error from a memory mutation (the gateway may throw on a
 * tool-level failure envelope, including a 'Checksum mismatch' message).
 */
export function classifyMemoryError(error: unknown): MemoryOutcome {
  const message = error instanceof Error ? error.message : String(error);
  if (CHECKSUM_MISMATCH.test(message)) return { kind: 'stale' };
  return { kind: 'error', message };
}

// ---------------------------------------------------------------------------
// Impure bridge calls
// ---------------------------------------------------------------------------

export async function searchExistingMemories(query: string): Promise<MemorySearchResult[]> {
  const trimmed = query.trim();
  if (!trimmed) return [];
  const response = await executeGateway<{ results?: MemorySearchResult[] }>(
    MEMORY_DOMAIN,
    'SearchMemories',
    {
      query: trimmed,
      mode: 'Hybrid',
      page: 1,
      pageSize: SEARCH_PAGE_SIZE,
      semanticWeight: 1,
      textWeight: 1,
      rrfK: 30,
    }
  );
  return response.results ?? [];
}

/** Read a memory note WITH its checksum (required for safe edits). */
export async function readMemoryWithChecksum(identifier: string): Promise<MemoryFile> {
  return executeGateway<MemoryFile>(MEMORY_DOMAIN, 'ReadMemory', {
    identifier,
    includeChecksum: true,
  });
}

/** Run a memory mutation and classify the outcome from raw text or a throw. */
async function runMutation(
  command: 'WriteMemory' | 'EditMemory',
  parameters: Record<string, unknown>
): Promise<MemoryOutcome> {
  try {
    const raw = await executeGatewayText(MEMORY_DOMAIN, command, parameters);
    return classifyMemoryOutcome(raw);
  } catch (error) {
    return classifyMemoryError(error);
  }
}

export async function writeMemory(request: WriteMemoryRequest): Promise<MemoryOutcome> {
  return runMutation('WriteMemory', { ...request });
}

export async function editMemory(request: EditMemoryRequest): Promise<MemoryOutcome> {
  const params: Record<string, unknown> = {
    identifier: request.identifier,
    operation: request.operation,
    content: request.content,
  };
  if (request.checksum) params.checksum = request.checksum;
  if (request.findText !== undefined) params.findText = request.findText;
  if (request.sectionName !== undefined) params.sectionName = request.sectionName;
  if (request.expectedCount !== undefined) params.expectedCount = request.expectedCount;
  return runMutation('EditMemory', params);
}
