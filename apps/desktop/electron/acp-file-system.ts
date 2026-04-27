import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { ipcMain } from 'electron';

interface ReadTextFileParams {
  path?: unknown;
  line?: unknown;
  limit?: unknown;
}

interface WriteTextFileParams {
  path?: unknown;
  content?: unknown;
}

function requireAbsolutePath(path: unknown): string {
  if (typeof path !== 'string' || path.length === 0) {
    throw new Error('ACP file path must be a non-empty string');
  }
  if (!isAbsolute(path)) {
    throw new Error('ACP file path must be absolute');
  }
  return path;
}

function optionalInteger(value: unknown, name: string): number | null {
  if (value == null) return null;
  if (!Number.isInteger(value)) {
    throw new Error(`ACP ${name} must be an integer`);
  }
  return value;
}

function sliceContent(content: string, line: number | null, limit: number | null): string {
  if (line === null && limit === null) return content;
  if (line !== null && line < 1) {
    throw new Error('ACP read line must be 1-based');
  }
  if (limit !== null && limit < 0) {
    throw new Error('ACP read limit must be non-negative');
  }
  const start = line === null ? 0 : line - 1;
  const lines = content.split('\n');
  const selected = limit === null ? lines.slice(start) : lines.slice(start, start + limit);
  return selected.join('\n');
}

export function registerAcpFileSystemHandlers(): void {
  ipcMain.handle('acp:read-text-file', async (_event, params: ReadTextFileParams) => {
    const filePath = requireAbsolutePath(params?.path);
    const line = optionalInteger(params?.line, 'read line');
    const limit = optionalInteger(params?.limit, 'read limit');
    const content = await readFile(filePath, 'utf8');
    return { content: sliceContent(content, line, limit) };
  });

  ipcMain.handle('acp:write-text-file', async (_event, params: WriteTextFileParams) => {
    const filePath = requireAbsolutePath(params?.path);
    if (typeof params?.content !== 'string') {
      throw new Error('ACP write content must be a string');
    }
    await writeFile(filePath, params.content, 'utf8');
    return {};
  });
}
