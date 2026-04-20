import { execSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, dialog, ipcMain } from 'electron';

// The desktop app connects to the maenifold + gate services running under
// launchd. It does NOT spawn its own daemons — one process per service,
// one tray icon total. If a service isn't running, we start it via launchctl.
const MAENIFOLD_SERVICE = 'com.ma-collective.maenifold';
const GATE_SERVICE = 'com.ma-collective.gate';
const MAENIFOLD_HEALTH_URL = 'http://localhost:8080/api/config';
const GATE_HEALTH_URL = 'http://localhost:8081/mcp';
const moduleDir = dirname(fileURLToPath(import.meta.url));

let mainWindow: BrowserWindow | null = null;
let isQuitting = false;

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function isServiceHealthy(url: string): Promise<boolean> {
  try {
    const response = await fetch(url);
    return response.status >= 200 && response.status < 500;
  } catch {
    return false;
  }
}

function startLaunchctlService(label: string): void {
  try {
    execSync(`launchctl start ${label}`, { timeout: 5_000 });
  } catch {
    console.error(`[maenifold] Failed to start ${label} via launchctl`);
  }
}

async function ensureServices(): Promise<void> {
  // Check maenifold on 8080
  if (!(await isServiceHealthy(MAENIFOLD_HEALTH_URL))) {
    console.log('[maenifold] Service not responding on 8080, starting via launchctl...');
    startLaunchctlService(MAENIFOLD_SERVICE);
  }
  // Check gate on 8081
  if (!(await isServiceHealthy(GATE_HEALTH_URL))) {
    console.log('[maenifold] Gate service not responding on 8081, starting via launchctl...');
    startLaunchctlService(GATE_SERVICE);
  }
  // Wait for maenifold to be healthy
  for (let attempt = 0; attempt < 30; attempt += 1) {
    if (await isServiceHealthy(MAENIFOLD_HEALTH_URL)) return;
    await wait(1_000);
  }
  console.error(
    '[maenifold] Service not responding on port 8080 after 30s. ' +
    'Check: launchctl print gui/$(id -u)/com.ma-collective.maenifold'
  );
}

async function createWindow(): Promise<void> {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    show: false,
    webPreferences: {
      preload: join(moduleDir, 'preload.mjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });
  mainWindow.on('ready-to-show', () => mainWindow?.show());
  mainWindow.on('close', (event) => {
    if (process.platform === 'darwin' && !isQuitting) {
      event.preventDefault();
      mainWindow?.hide();
    }
  });

  const rendererUrl = process.env.ELECTRON_RENDERER_URL ?? process.env.VITE_DEV_SERVER_URL;
  if (rendererUrl) await mainWindow.loadURL(rendererUrl);
  else await mainWindow.loadFile(join(app.getAppPath(), 'dist', 'index.html'));
}

function registerIpcHandlers(): void {
  ipcMain.handle('dialog:open-directory', async () => {
    const result = await dialog.showOpenDialog(mainWindow ?? undefined, { properties: ['openDirectory'] });
    return result.canceled ? null : (result.filePaths[0] ?? null);
  });
  ipcMain.handle('dialog:open-file', async () => {
    const result = await dialog.showOpenDialog(mainWindow ?? undefined, { properties: ['openFile'] });
    return result.canceled ? null : (result.filePaths[0] ?? null);
  });
  ipcMain.handle('file:read-binary', async (_event, filePath: string) => readFile(filePath));
  ipcMain.handle('app:get-version', () => app.getVersion());
}

app.on('render-process-gone', async (_event, _webContents, details) => {
  console.error('[maenifold] Renderer process gone:', details.reason, details.exitCode);
  if (details.reason !== 'clean-exit') {
    mainWindow = null;
    await createWindow();
  }
});

app.on('child-process-gone', (_event, details) => {
  console.error('[maenifold] Child process gone:', details.type, details.reason);
});

process.on('uncaughtException', (error) => {
  console.error('[maenifold] Uncaught exception:', error);
  const fs = require('fs');
  const path = require('path');
  const crashLog = path.join(app.getPath('userData'), 'crash.log');
  fs.appendFileSync(crashLog, `[${new Date().toISOString()}] uncaughtException: ${error.stack || error}\n`);
  process.exit(1);
});

process.on('unhandledRejection', (reason) => {
  console.error('[maenifold] Unhandled rejection:', reason);
});

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.show();
    mainWindow.focus();
  });
  app.on('before-quit', () => {
    isQuitting = true;
  });
  app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') app.quit();
  });
  app.on('activate', async () => {
    if (BrowserWindow.getAllWindows().length === 0) await createWindow();
    else mainWindow?.show();
  });
  app.whenReady().then(async () => {
    registerIpcHandlers();
    await ensureServices();
    await createWindow();
  });
}
