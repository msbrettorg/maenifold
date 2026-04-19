// Pure helpers and types for the knowledge store. Extracted from
// stores/knowledge.ts to keep modules under 300 LOC.
import type { MemoryFile } from './workflow-types';

export interface KnowledgeFileEntry {
  name: string;
  path: string;
  uri: string;
}

export interface KnowledgeFolderNode {
  name: string;
  path: string;
  fileCount: number;
  open: boolean;
  loaded: boolean;
  loading: boolean;
  error: string | null;
  children: KnowledgeFolderNode[];
  files: KnowledgeFileEntry[];
}

export interface KnowledgeConceptNode {
  name: string;
  occurrences: number;
  edgeCount: number;
}

export interface ParsedListing {
  folders: Array<{ name: string; path: string; fileCount: number }>;
  files: KnowledgeFileEntry[];
}

export function trimPath(value: string): string {
  return value.trim().replace(/^\/+|\/+$/g, '');
}

export function joinPath(base: string, segment: string): string {
  const normalizedBase = trimPath(base);
  const normalizedSegment = trimPath(segment);
  if (!normalizedBase) return normalizedSegment;
  if (!normalizedSegment) return normalizedBase;
  return `${normalizedBase}/${normalizedSegment}`;
}

export function buildMemoryUri(path: string, fileName: string): string {
  const stem = fileName.replace(/\.md$/i, '');
  const targetPath = joinPath(path, stem);
  return `memory://${targetPath}`;
}

export function parseListMemoriesOutput(output: string, currentPath: string): ParsedListing {
  const folders: ParsedListing['folders'] = [];
  const files: ParsedListing['files'] = [];
  const lines = output.split('\n');
  let section: 'folders' | 'files' | null = null;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    if (trimmed === '### Folders') {
      section = 'folders';
      continue;
    }

    if (trimmed === '### Files') {
      section = 'files';
      continue;
    }

    if (section === 'folders') {
      const match = trimmed.match(/^📁\s+(.+?)\/\s+\((\d+)\s+files\)$/);
      if (!match) continue;
      const folderName = match[1] ?? '';
      const fileCount = Number(match[2] ?? 0);
      folders.push({
        name: folderName,
        path: joinPath(currentPath, folderName),
        fileCount: Number.isFinite(fileCount) ? fileCount : 0,
      });
      continue;
    }

    if (section === 'files') {
      const match = trimmed.match(/^📄\s+(.+?)\s+\([^)]+\)$/);
      if (!match) continue;
      const fileName = (match[1] ?? '').trim();
      files.push({
        name: fileName,
        path: joinPath(currentPath, fileName),
        uri: buildMemoryUri(currentPath, fileName),
      });
    }
  }

  return { folders, files };
}

export function createFolderNode(
  input: ParsedListing['folders'][number],
  open = false,
): KnowledgeFolderNode {
  return {
    name: input.name,
    path: input.path,
    fileCount: input.fileCount,
    open,
    loaded: false,
    loading: false,
    error: null,
    children: [],
    files: [],
  };
}

export function findFolderNode(
  folders: KnowledgeFolderNode[],
  targetPath: string,
): KnowledgeFolderNode | null {
  for (const folder of folders) {
    if (folder.path === targetPath) return folder;
    const nested = findFolderNode(folder.children, targetPath);
    if (nested) return nested;
  }
  return null;
}

export function normalizeConceptName(rawValue: string): string {
  const normalized = rawValue.trim();
  const aliasIndex = normalized.indexOf('|');
  if (aliasIndex > 0) {
    return normalized.slice(0, aliasIndex).trim();
  }
  return normalized;
}

export function extractConceptCandidates(note: MemoryFile | null): string[] {
  if (!note) return [];

  const fromWikiLinks = [...note.content.matchAll(/\[\[([^\]]+)\]\]/g)]
    .map((match) => normalizeConceptName(match[1] ?? ''))
    .filter((value) => value.length > 0);

  if (fromWikiLinks.length > 0) return fromWikiLinks;

  const fromTitle = note.title.trim();
  if (fromTitle.length > 0 && !fromTitle.startsWith('memory://')) {
    return [fromTitle];
  }

  const uriPath = note.uri.replace(/^memory:\/\//, '');
  const uriStem = uriPath.split('/').at(-1)?.trim() ?? '';
  if (!uriStem) return [];

  return [uriStem.replace(/[-_]+/g, ' ')];
}

export function findBestAnchorUri(conceptName: string, files: string[]): string | null {
  const memoryUris = files.filter((file) => file.startsWith('memory://'));
  if (memoryUris.length === 0) return null;

  const conceptSlug = conceptName.toLowerCase().replace(/\s+/g, '-');
  const anchored = memoryUris.find((uri) => uri.toLowerCase().includes(conceptSlug));
  return anchored ?? memoryUris[0] ?? null;
}
