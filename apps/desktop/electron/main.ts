import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, dialog, ipcMain } from 'electron';

const DAEMON_HEALTH_URL = 'http://localhost:19760/api/config';
const moduleDir = dirname(fileURLToPath(import.meta.url));

let mainWindow: BrowserWindow | null = null;
let daemonProcess: ChildProcessWithoutNullStreams | null = null;
let isQuitting = false;
let hasHandledBeforeQuit = false;
let expectedDaemonExit = false;
let startPromise: Promise<void> | null = null;

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

function resolveDaemonBinary(): string {
  const binaryName = process.platform === 'win32' ? 'maenifold.exe' : 'maenifold';
  const candidates = [
    join(process.resourcesPath, binaryName),
    join(dirname(app.getPath('exe')), binaryName),
    join(process.resourcesPath, 'bin', binaryName),
  ];
  return candidates.find((candidate) => existsSync(candidate)) ?? binaryName;
}

async function waitForDaemon(): Promise<void> {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    if (!daemonProcess) throw new Error('maenifold exited before becoming healthy');
    try {
      const response = await fetch(DAEMON_HEALTH_URL);
      if (response.status === 200) return;
    } catch {
      // Retry until the daemon is ready.
    }
    await wait(500);
  }
  throw new Error('Timed out waiting for maenifold daemon health check');
}

function attachDaemonLifecycle(child: ChildProcessWithoutNullStreams): void {
  child.stderr.on('data', (chunk) => console.error(`[maenifold] ${chunk}`.trimEnd()));
  child.on('exit', () => {
    daemonProcess = null;
    startPromise = null;
    if (expectedDaemonExit) {
      expectedDaemonExit = false;
      return;
    }
    if (!isQuitting) void startDaemon();
  });
}

async function stopDaemon(): Promise<void> {
  if (!daemonProcess) return;
  expectedDaemonExit = true;
  const child = daemonProcess;
  await new Promise<void>((resolve) => {
    child.once('exit', () => resolve());
    child.kill('SIGTERM');
    setTimeout(() => {
      if (child.exitCode === null) child.kill('SIGKILL');
    }, 3_000);
  });
}

async function startDaemon(): Promise<void> {
  if (startPromise) return startPromise;
  startPromise = (async () => {
    if (daemonProcess) return;
    daemonProcess = spawn(resolveDaemonBinary(), ['--http', '--port', '19760'], {
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    attachDaemonLifecycle(daemonProcess);
    await waitForDaemon();
  })().finally(() => {
    startPromise = null;
  });
  return startPromise;
}

async function restartDaemon(): Promise<void> {
  await stopDaemon();
  await startDaemon();
}

async function createWindow(): Promise<void> {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    show: false,
    webPreferences: {
      preload: join(moduleDir, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
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
  ipcMain.handle('daemon:restart', async () => restartDaemon());
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.show();
    mainWindow.focus();
  });
  app.on('before-quit', (event) => {
    isQuitting = true;
    if (!daemonProcess || hasHandledBeforeQuit) return;
    hasHandledBeforeQuit = true;
    event.preventDefault();
    void stopDaemon().finally(() => app.quit());
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
    await startDaemon();
    await createWindow();
  });
}
