import { execFileSync } from 'node:child_process';
import { appendFileSync, existsSync, lstatSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, dialog, ipcMain, protocol, shell } from 'electron';
import { registerAcpFileSystemHandlers } from './acp-file-system.js';
import { registerConfigHandlers } from './config-file.js';
import { registerUserPromptBridge } from './user-prompt-bridge.js';
import { registerVec0Handlers } from './vec0.js';
import { registerVoiceHandlers } from './voice.js';
import { registerPermissionLedgerHandlers } from './permission-ledger.js';
import {
  TerminalManager,
  validateCreateTerminalParams,
  validateTerminalOperationParams,
} from './terminalManager.js';

function ignoreOnlyEpipe(error: NodeJS.ErrnoException) {
  if (error.code === 'EPIPE') return;
  try { appendFileSync(join(app.getPath('userData'), 'crash.log'), `[${new Date().toISOString()}] stdio error: ${error.stack || error}\n`); } catch { /* pre-ready or write failure — nothing we can do */ }
}
process.stdout?.on('error', ignoreOnlyEpipe);
process.stderr?.on('error', ignoreOnlyEpipe);

const writeConsoleError = (...args: unknown[]) => console['error'](...args);
function safeLogError(...args: unknown[]) {
  try { writeConsoleError(...args); } catch { /* crash-path: logging failure is non-fatal */ }
}
function safeLogInfo(message: string) {
  try { process.stdout.write(`${message}\n`); } catch { /* crash-path: logging failure is non-fatal */ }
}

protocol.registerSchemesAsPrivileged([
  { scheme: 'mcp-app-sandbox', privileges: { standard: true, secure: true, supportFetchAPI: true } },
]);
const MAENIFOLD_SERVICE = 'com.ma-collective.maenifold';
const MAENIFOLD_HEALTH_URL = 'http://localhost:8080/api/config';
const moduleDir = dirname(fileURLToPath(import.meta.url));
const launchctlDomain = process.platform === 'darwin' && typeof process.getuid === 'function'
  ? `gui/${process.getuid()}`
  : null;
let mainWindow: BrowserWindow | null = null;
let isQuitting = false;
let selectedGateRoot: string | null = null;
const terminalManager = new TerminalManager(() => mainWindow?.webContents ?? null);
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const gateTaskFilenamePattern = /^T-\d+\.\d+-[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;

type GateWriteTaskFileParams = { root?: unknown; filename?: unknown; content?: unknown };
type GateSetRootParams = { root?: unknown };

function validateGateWriteTaskFileParams(params: unknown) {
  if (!params || typeof params !== 'object') throw new Error('parameters are required');
  const candidate = params as GateWriteTaskFileParams;
  if (typeof candidate.root !== 'string' || candidate.root.length === 0) throw new Error('root is required');
  if (typeof candidate.filename !== 'string' || !gateTaskFilenamePattern.test(candidate.filename)) {
    throw new Error('invalid task filename');
  }
  if (typeof candidate.content !== 'string') throw new Error('content is required');
  return { root: candidate.root, filename: candidate.filename, content: candidate.content };
}

function validateGateSetRootParams(params: unknown) {
  if (!params || typeof params !== 'object') throw new Error('parameters are required');
  const candidate = params as GateSetRootParams;
  if (candidate.root !== null && typeof candidate.root !== 'string') throw new Error('root is required');
  if (candidate.root !== null && !isAbsolute(candidate.root)) throw new Error('gate:set-root requires an absolute path');
  return { root: candidate.root as string | null };
}

async function isServiceHealthy(url: string): Promise<boolean> {
  try {
    const response = await fetch(url);
    return response.status >= 200 && response.status < 500;
  } catch { return false; }
}
function launchAgentPath(label: string): string { return join(app.getPath('home'), 'Library', 'LaunchAgents', `${label}.plist`); }
function maenifoldRoot(): string { return process.env.MAENIFOLD_ROOT || join(app.getPath('home'), 'maenifold'); }
function isLaunchctlServiceLoaded(label: string): boolean {
  if (!launchctlDomain) return false;
  try {
    execFileSync('launchctl', ['print', `${launchctlDomain}/${label}`], { stdio: 'ignore', timeout: 5_000 });
    return true;
  } catch { return false; }
}
function startLaunchctlService(label: string): void {
  if (!launchctlDomain) return;
  const plistPath = launchAgentPath(label);
  try {
    if (!isLaunchctlServiceLoaded(label)) {
      if (!existsSync(plistPath)) {
        safeLogError(`[maenifold] LaunchAgent missing for ${label}: ${plistPath}`);
        return;
      }
      execFileSync('launchctl', ['bootstrap', launchctlDomain, plistPath], { timeout: 5_000 });
    }
    execFileSync('launchctl', ['kickstart', '-k', `${launchctlDomain}/${label}`], { timeout: 5_000 });
  } catch (error) {
    safeLogError(`[maenifold] Failed to start ${label} via launchctl`, error);
  }
}
async function ensureServices(): Promise<void> {
  if (!(await isServiceHealthy(MAENIFOLD_HEALTH_URL))) {
    safeLogInfo('[maenifold] Service not responding on 8080, starting via launchctl...');
    startLaunchctlService(MAENIFOLD_SERVICE);
  }
  for (let attempt = 0; attempt < 30; attempt += 1) {
    if (await isServiceHealthy(MAENIFOLD_HEALTH_URL)) return;
    await wait(1_000);
  }
  safeLogError(
    '[maenifold] Service not responding on port 8080 after 30s. ' +
      'Check: launchctl print gui/$(id -u)/com.ma-collective.maenifold',
  );
}
async function createWindow(): Promise<void> {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    show: false,
    webPreferences: { preload: join(moduleDir, 'preload.mjs'), contextIsolation: true, nodeIntegration: false, sandbox: true },
  });
  mainWindow.on('ready-to-show', () => mainWindow?.show());
  mainWindow.on('close', (event) => { if (process.platform === 'darwin' && !isQuitting) { event.preventDefault(); mainWindow?.hide(); } });
  const rendererOrigin = process.env.ELECTRON_RENDERER_URL
    ? (() => {
        try { return new URL(process.env.ELECTRON_RENDERER_URL).origin; } catch { return null; }
      })()
    : null;
  const appPath = app.getAppPath();
  mainWindow.webContents.on('will-navigate', (event, url) => {
    try {
      const parsed = new URL(url);
      if (parsed.protocol === 'file:' && parsed.pathname.startsWith(appPath)) return;
      if (rendererOrigin && parsed.origin === rendererOrigin) return;
    } catch {}
    event.preventDefault();
  });
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    try { if (['http:', 'https:'].includes(new URL(url).protocol)) void shell.openExternal(url); } catch {}
    return { action: 'deny' };
  });
  mainWindow.webContents.session.webRequest.onHeadersReceived((details, callback) => {
    if (details.url.startsWith('mcp-app-sandbox://')) {
      callback({ responseHeaders: details.responseHeaders });
      return;
    }
    callback({ responseHeaders: { ...details.responseHeaders, 'Content-Security-Policy': ["default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; media-src 'self' data:; connect-src 'self' http://localhost:* http://127.0.0.1:* ws://localhost:* ws://127.0.0.1:*; font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none';"] } });
  });
  mainWindow.webContents.session.setPermissionRequestHandler((_webContents, permission, callback, details) => {
    const isSandbox = details.requestingUrl?.startsWith('mcp-app-sandbox://');
    if (isSandbox && new Set(['clipboard-write', 'clipboard-read']).has(permission)) { callback(true); return; }
    callback(false);
  });
  const rendererUrl = process.env.ELECTRON_RENDERER_URL ?? process.env.VITE_DEV_SERVER_URL;
  if (rendererUrl) await mainWindow.loadURL(rendererUrl);
  else await mainWindow.loadFile(join(app.getAppPath(), 'dist', 'index.html'));
}
function registerSandboxProtocol(): void {
  protocol.handle('mcp-app-sandbox', (request) => {
    const url = new URL(request.url);
    let cspHeader = "default-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval' blob: data:; style-src 'self' 'unsafe-inline' blob: data:; img-src 'self' data: blob:; font-src 'self' data: blob:; connect-src 'self'; worker-src 'self' blob:; frame-src 'none'; object-src 'none'; base-uri 'none'";
    const cspParam = url.searchParams.get('csp');
    if (cspParam) {
      try {
        const cspConfig = JSON.parse(decodeURIComponent(cspParam));
        if (Array.isArray(cspConfig?.connectDomains)) {
          const safe = cspConfig.connectDomains.filter(
            (domain: unknown) => typeof domain === 'string' && !/[;\r\n'" ]/.test(domain),
          );
          if (safe.length) cspHeader = cspHeader.replace("connect-src 'self'", `connect-src 'self' ${safe.join(' ')}`);
        }
      } catch {}
    }
    const sandboxHtml = `<!doctype html>
<html><head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>html,body{margin:0;height:100vh;width:100vw;background:transparent}body{display:flex;flex-direction:column}*{box-sizing:border-box}iframe{background:transparent;border:0;padding:0;overflow:hidden;flex-grow:1;color-scheme:inherit}</style>
</head><body><script type="module">
import{buildAllowAttribute}from"@modelcontextprotocol/ext-apps/app-bridge";
const inner=document.createElement("iframe");
inner.style="width:100%;height:100%;border:none";
inner.setAttribute("sandbox","allow-scripts allow-same-origin allow-forms");
document.body.appendChild(inner);
window.addEventListener("message",e=>{
  if(e.source===window.parent){
    if(e.data?.method==="ui/notifications/sandbox-resource-ready"){
      const{html,sandbox,permissions}=e.data.params;
      if(typeof sandbox==="string")inner.setAttribute("sandbox",sandbox);
      const allow=buildAllowAttribute(permissions);
      if(allow)inner.setAttribute("allow",allow);
      if(typeof html==="string"){
        const doc=inner.contentDocument||inner.contentWindow?.document;
        if(doc){doc.open();doc.write(html);doc.close()}
        else{inner.srcdoc=html}
      }
    }else if(inner.contentWindow){
      inner.contentWindow.postMessage(e.data,"*");
    }
  }else if(e.source===inner.contentWindow){
    window.parent.postMessage(e.data,"*");
  }
});
window.parent.postMessage({jsonrpc:"2.0",method:"ui/notifications/sandbox-proxy-ready",params:{}},"*");
<\/script></body></html>`;
    return new Response(sandboxHtml, {
      headers: { 'Content-Type': 'text/html; charset=utf-8', 'Content-Security-Policy': cspHeader, 'Cache-Control': 'no-cache' },
    });
  });
}
export async function openDirectoryDialog(): Promise<string | null> {
  if (process.env.MAENIFOLD_UAT === '1') {
    return process.env.MAENIFOLD_UAT_FOLDER_PICKER_PATH ?? null;
  }
  const ownerWindow = mainWindow ?? BrowserWindow.getFocusedWindow();
  const result = ownerWindow
    ? await dialog.showOpenDialog(ownerWindow, { properties: ['openDirectory'] })
    : await dialog.showOpenDialog({ properties: ['openDirectory'] });
  return result.canceled ? null : (result.filePaths[0] ?? null);
}
function registerIpcHandlers(): void {
  ipcMain.handle('dialog:open-directory', () => openDirectoryDialog());
  ipcMain.handle('app:get-version', () => app.getVersion());
  ipcMain.handle('app:quit', () => { app.quit(); });
  ipcMain.handle('check-env-vars', (_event, names: string[]) => {
    const result: Record<string, boolean> = {};
    for (const name of names) result[name] = !!process.env[name];
    return result;
  });
  ipcMain.handle('gate:set-root', (_event, params: unknown) => {
    const { root } = validateGateSetRootParams(params);
    selectedGateRoot = root === null ? null : realpathSync(root);
    return {};
  });
  ipcMain.handle('gate:write-task-file', (_event, params: unknown) => {
    const { root, filename, content } = validateGateWriteTaskFileParams(params);
    const resolvedRoot = realpathSync(root);
    if (selectedGateRoot === null || resolvedRoot !== realpathSync(selectedGateRoot)) throw new Error('Root mismatch: provided root does not match selected gate root');
    const todoDir = join(root, 'todo');
    if (lstatSync(todoDir).isSymbolicLink()) throw new Error('Refusing to write: todo/ is a symbolic link');
    const resolvedTodo = realpathSync(todoDir);
    if (!resolvedTodo.startsWith(`${resolvedRoot}/`)) throw new Error('Refusing to write: todo/ resolves outside root');
    const taskPath = join(todoDir, filename);
    writeFileSync(taskPath, content, { flag: 'wx' });
    return { path: taskPath };
  });
  ipcMain.handle('terminal:create', (_event, params) => terminalManager.createTerminal(validateCreateTerminalParams(params)));
  ipcMain.handle('terminal:output', (_event, params) => terminalManager.getOutput(validateTerminalOperationParams(params).terminalId));
  ipcMain.handle('terminal:wait_for_exit', (_event, params) => terminalManager.waitForExit(validateTerminalOperationParams(params).terminalId));
  ipcMain.handle('terminal:kill', (_event, params) => terminalManager.kill(validateTerminalOperationParams(params).terminalId));
  ipcMain.handle('terminal:release', (_event, params) => terminalManager.release(validateTerminalOperationParams(params).terminalId));
  registerAcpFileSystemHandlers();
  registerConfigHandlers();
  registerUserPromptBridge(() => mainWindow?.webContents ?? null);
  registerVec0Handlers();
  registerVoiceHandlers();
  registerPermissionLedgerHandlers();
}
app.on('render-process-gone', async (_event, _webContents, details) => {
  safeLogError('[maenifold] Renderer process gone:', details.reason, details.exitCode);
  if (details.reason !== 'clean-exit') { mainWindow = null; terminalManager.dispose(); await createWindow(); }
});
app.on('child-process-gone', (_event, details) => safeLogError('[maenifold] Child process gone:', details.type, details.reason));
process.on('uncaughtException', (error) => {
  if ((error as NodeJS.ErrnoException).code === 'EPIPE') return;
  safeLogError('[maenifold] Uncaught exception:', error);
  appendFileSync(join(app.getPath('userData'), 'crash.log'), `[${new Date().toISOString()}] uncaughtException: ${error.stack || error}\n`);
  terminalManager.dispose();
  setImmediate(() => process.exit(1));
});
process.on('unhandledRejection', (reason) => safeLogError('[maenifold] Unhandled rejection:', reason));
app.on('before-quit', () => { isQuitting = true; terminalManager.dispose(); });
app.on('will-quit', () => { terminalManager.dispose(); });
app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
app.on('activate', async () => { if (BrowserWindow.getAllWindows().length === 0) await createWindow(); else mainWindow?.show(); });
app.whenReady().then(async () => {
  if (process.env.MAENIFOLD_UAT === '1') safeLogInfo('[uat] MAENIFOLD_UAT mode active - native dialogs return synthetic results');
  registerIpcHandlers();
  registerSandboxProtocol();
  await ensureServices();
  await createWindow();
});
