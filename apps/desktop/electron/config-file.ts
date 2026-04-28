// Maenifold config file IPC. Settings tab is the sole owner of ~/maenifold/maenifold.json.
// Daemon hot-reloads via its own file watcher; the desktop never goes through MCP for config writes.
//
// Reference: https://www.electronjs.org/docs/latest/api/ipc-main
import { watch, type FSWatcher } from 'node:fs';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { app, BrowserWindow, ipcMain } from 'electron';

function configRoot(): string {
  return process.env.MAENIFOLD_ROOT || join(homedir(), 'maenifold');
}

function configPath(): string {
  return join(configRoot(), 'maenifold.json');
}

async function readConfig(): Promise<{ path: string; content: string; exists: boolean }> {
  const path = configPath();
  try {
    const content = await readFile(path, 'utf8');
    return { path, content, exists: true };
  } catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') {
      return { path, content: '{\n  "mcpServers": [],\n  "agents": {}\n}\n', exists: false };
    }
    throw error;
  }
}

async function writeConfig(content: string): Promise<{ path: string }> {
  if (typeof content !== 'string' || content.length === 0) {
    throw new Error('config:write content must be a non-empty string');
  }
  // Validate JSON before writing to keep the daemon out of a poison state.
  try {
    JSON.parse(content);
  } catch (error) {
    throw new Error(`config:write rejected invalid JSON: ${(error as Error).message}`);
  }
  const path = configPath();
  await mkdir(dirname(path), { recursive: true });
  // Atomic write: stage to a sibling temp file, then rename.
  const stagedPath = `${path}.tmp-${process.pid}-${Date.now()}`;
  await writeFile(stagedPath, content, 'utf8');
  await rename(stagedPath, path);
  return { path };
}

let watcher: FSWatcher | null = null;

function startWatcher(): void {
  if (watcher) return;
  const path = configPath();
  try {
    watcher = watch(path, { persistent: false }, (eventType) => {
      if (eventType !== 'change' && eventType !== 'rename') return;
      for (const window of BrowserWindow.getAllWindows()) {
        window.webContents.send('config:changed');
      }
    });
  } catch (error) {
    console.error('[maenifold] config watcher failed', error);
  }
}

function stopWatcher(): void {
  watcher?.close();
  watcher = null;
}

export function registerConfigHandlers(): void {
  ipcMain.handle('config:read', async () => readConfig());
  ipcMain.handle('config:write', async (_event, content: string) => writeConfig(content));
  ipcMain.handle('config:path', async () => configPath());
  app.on('will-quit', stopWatcher);
  startWatcher();
}
