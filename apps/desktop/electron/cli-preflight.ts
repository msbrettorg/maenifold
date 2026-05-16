import { spawnSync } from 'node:child_process';
import { ipcMain } from 'electron';

function extractBinary(command: string): string {
  const trimmed = command.trim();
  if (!trimmed) return '';
  const tokens = trimmed.split(/\s+/);
  if (tokens[0] === 'npx' || tokens[0] === 'bunx') {
    return tokens[1] ?? '';
  }
  return tokens[0];
}

function checkBinaryOnPath(binary: string): { found: boolean; path: string | null } {
  if (!binary) return { found: false, path: null };
  try {
    const isWin = process.platform === 'win32';
    const result = isWin
      ? spawnSync('where.exe', [binary], { timeout: 5000, encoding: 'utf8' })
      : spawnSync('/bin/sh', ['-c', 'command -v -- "$1"', 'sh', binary], {
        timeout: 5000,
        encoding: 'utf8',
      });
    if (result.status === 0 && result.stdout) {
      return { found: true, path: result.stdout.trim().split('\n')[0] };
    }
    return { found: false, path: null };
  } catch {
    return { found: false, path: null };
  }
}

export function registerCliPreflightHandlers(): void {
  ipcMain.handle('cli:check-binary', (_event, command: unknown) => {
    if (typeof command !== 'string') return { found: false, path: null };
    const binary = extractBinary(command);
    return checkBinaryOnPath(binary);
  });
}
