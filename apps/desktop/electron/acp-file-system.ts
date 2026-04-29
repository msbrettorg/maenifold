// T-238.33: ACP file I/O has an accepted TOCTOU residual risk.
// A same-user local attacker who can mutate ancestor directories under memory/
// can race between path validation and read/write I/O by swapping symlinks or
// directories after validation completes. At that privilege level the attacker
// can already read/modify those files directly, so this does not escalate
// privileges. Node.js does not expose race-free openat(2)-style traversal
// without a native addon, so we cannot fully eliminate the race here. We still
// realpath-check targets before I/O to reject existing symlink traversal; write
// validation resolves the parent for new files and restricts writes to
// ~/maenifold/memory. This raises the bar but remains pre-I/O validation rather
// than a race-free guarantee. Risk formally accepted.
import { readFile, realpath, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import * as path from 'node:path';
import { ipcMain } from 'electron';
import {
  consumeWriteToken,
  isAlwaysAllowed,
  issueWriteToken,
  rememberAlwaysAllowed,
} from './write-permission-tokens.js';

const MAENIFOLD_MEMORY_ROOT = path.join(homedir(), 'maenifold', 'memory');

interface ReadTextFileParams {
  path?: unknown;
  line?: unknown;
  limit?: unknown;
}

interface WriteTextFileParams {
  path?: unknown;
  content?: unknown;
  token?: unknown;
}

class WritePermissionRejectedError extends Error {
  constructor(filePath: string, reason: string) {
    super(`Write to ${filePath} rejected: ${reason}`);
    this.name = 'WritePermissionRejectedError';
  }
}

function requireAbsolutePath(filePath: unknown): string {
  if (typeof filePath !== 'string' || filePath.length === 0) {
    throw new Error('ACP file path must be a non-empty string');
  }
  if (!path.isAbsolute(filePath)) {
    throw new Error('ACP file path must be absolute');
  }
  return filePath;
}

async function requireNonSymlinkPath(filePath: string): Promise<string> {
  const resolvedPath = await realpath(filePath);
  if (resolvedPath !== filePath) {
    throw new Error('ACP file path resolves through a symlink, which is not allowed');
  }
  return resolvedPath;
}

async function requireNonSymlinkWritePath(filePath: string): Promise<string> {
  const parentPath = path.dirname(filePath);
  const normalizedParentPath = path.normalize(parentPath);
  const resolvedParentPath = await realpath(parentPath);
  if (resolvedParentPath !== normalizedParentPath) {
    throw new Error('ACP file path resolves through a symlink, which is not allowed');
  }

  try {
    const resolvedPath = await realpath(filePath);
    if (resolvedPath !== filePath) {
      throw new Error('ACP file path resolves through a symlink, which is not allowed');
    }
    return resolvedPath;
  } catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') {
      return path.join(resolvedParentPath, path.basename(filePath));
    }
    throw error;
  }
}

function requireWriteWithinMemory(filePath: string): void {
  const normalizedFilePath = path.normalize(filePath);
  const normalizedWriteRoot = path.normalize(MAENIFOLD_MEMORY_ROOT + path.sep);
  if (!normalizedFilePath.startsWith(normalizedWriteRoot)) {
    throw new Error('ACP writes are restricted to the maenifold memory directory');
  }
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
  ipcMain.handle('acp:write-permission:issue-token', async (_event, params: {
    path?: unknown;
    allowsAlways?: unknown;
  }) => {
    const filePath = requireAbsolutePath(params?.path);
    return issueWriteToken(filePath, params?.allowsAlways === true);
  });

  ipcMain.handle('acp:write-permission:check-always-allowed', async (_event, filePath: unknown) =>
    isAlwaysAllowed(requireAbsolutePath(filePath))
  );

  ipcMain.handle('acp:read-text-file', async (_event, params: ReadTextFileParams) => {
    const filePath = requireAbsolutePath(params?.path);
    const resolvedPath = await requireNonSymlinkPath(filePath);
    const line = optionalInteger(params?.line, 'read line');
    const limit = optionalInteger(params?.limit, 'read limit');
    const content = await readFile(resolvedPath, 'utf8');
    return { content: sliceContent(content, line, limit) };
  });

  ipcMain.handle('acp:write-text-file', async (_event, params: WriteTextFileParams) => {
    const filePath = requireAbsolutePath(params?.path);
    if (typeof params?.content !== 'string') {
      throw new Error('ACP write content must be a string');
    }
    if (!isAlwaysAllowed(filePath)) {
      if (typeof params?.token !== 'string') {
        throw new WritePermissionRejectedError(filePath, 'write requires permission token');
      }
      const tokenResult = consumeWriteToken(params.token, filePath);
      if (!tokenResult.ok) {
        throw new WritePermissionRejectedError(filePath, tokenResult.reason);
      }
      if (tokenResult.allowsAlways) {
        rememberAlwaysAllowed(filePath);
      }
    }
    const resolvedPath = await requireNonSymlinkWritePath(filePath);
    requireWriteWithinMemory(resolvedPath);
    await writeFile(resolvedPath, params.content, 'utf8');
    return {};
  });
}
