import { execFileSync } from 'node:child_process';
import { appendFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, dialog, ipcMain, protocol, shell } from 'electron';
import { registerAcpFileSystemHandlers } from './acp-file-system.js';
import { registerConfigHandlers } from './config-file.js';
import { registerVec0Handlers } from './vec0.js';
import { registerVoiceHandlers } from './voice.js';
import { registerPermissionLedgerHandlers } from './permission-ledger.js';
import { TerminalManager } from './terminalManager.js';
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
const terminalManager = new TerminalManager(() => mainWindow?.webContents ?? null);
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
async function isServiceHealthy(url: string): Promise<boolean> {
  try {
    const response = await fetch(url);
    return response.status >= 200 && response.status < 500;
  } catch {
    return false;
  }
}
function launchAgentPath(label: string): string {
  return join(app.getPath('home'), 'Library', 'LaunchAgents', `${label}.plist`);
}
function maenifoldRoot(): string {
  return process.env.MAENIFOLD_ROOT || join(app.getPath('home'), 'maenifold');
}
function isLaunchctlServiceLoaded(label: string): boolean {
  if (!launchctlDomain) return false;
  try {
    execFileSync('launchctl', ['print', `${launchctlDomain}/${label}`], { stdio: 'ignore', timeout: 5_000 });
    return true;
  } catch {
    return false;
  }
}
function startLaunchctlService(label: string): void {
  if (!launchctlDomain) return;
  const plistPath = launchAgentPath(label);
  try {
    if (!isLaunchctlServiceLoaded(label)) {
      if (!existsSync(plistPath)) {
        console.error(`[maenifold] LaunchAgent missing for ${label}: ${plistPath}`);
        return;
      }
      execFileSync('launchctl', ['bootstrap', launchctlDomain, plistPath], { timeout: 5_000 });
    }
    execFileSync('launchctl', ['kickstart', '-k', `${launchctlDomain}/${label}`], { timeout: 5_000 });
  } catch (error) {
    console.error(`[maenifold] Failed to start ${label} via launchctl`, error);
  }
}
async function ensureServices(): Promise<void> {
  if (!(await isServiceHealthy(MAENIFOLD_HEALTH_URL))) {
    console.log('[maenifold] Service not responding on 8080, starting via launchctl...');
    startLaunchctlService(MAENIFOLD_SERVICE);
  }
  for (let attempt = 0; attempt < 30; attempt += 1) {
    if (await isServiceHealthy(MAENIFOLD_HEALTH_URL)) return;
    await wait(1_000);
  }
  console.error(
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
  mainWindow.on('close', (event) => {
    if (process.platform === 'darwin' && !isQuitting) {
      event.preventDefault();
      mainWindow?.hide();
    }
  });
  const rendererOrigin = process.env.ELECTRON_RENDERER_URL
    ? (() => {
        try {
          return new URL(process.env.ELECTRON_RENDERER_URL).origin;
        } catch {
          return null;
        }
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
    try {
      if (['http:', 'https:'].includes(new URL(url).protocol)) void shell.openExternal(url);
    } catch {}
    return { action: 'deny' };
  });
  mainWindow.webContents.session.webRequest.onHeadersReceived((details, callback) => {
    if (details.url.startsWith('mcp-app-sandbox://')) {
      callback({ responseHeaders: details.responseHeaders });
      return;
    }
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' http://localhost:* http://127.0.0.1:* ws://localhost:* ws://127.0.0.1:*; font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none';",
        ],
      },
    });
  });
  mainWindow.webContents.session.setPermissionRequestHandler((_webContents, permission, callback, details) => {
    const isSandbox = details.requestingUrl?.startsWith('mcp-app-sandbox://');
    if (isSandbox && new Set(['clipboard-write', 'clipboard-read']).has(permission)) {
      callback(true);
      return;
    }
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
function registerIpcHandlers(): void {
  ipcMain.handle('dialog:open-directory', async () => {
    const ownerWindow = mainWindow ?? BrowserWindow.getFocusedWindow();
    const result = ownerWindow
      ? await dialog.showOpenDialog(ownerWindow, { properties: ['openDirectory'] })
      : await dialog.showOpenDialog({ properties: ['openDirectory'] });
    return result.canceled ? null : (result.filePaths[0] ?? null);
  });
  ipcMain.handle('app:get-version', () => app.getVersion());
  ipcMain.handle('check-env-vars', (_event, names: string[]) => {
    const result: Record<string, boolean> = {};
    for (const name of names) {
      result[name] = !!process.env[name];
    }
    return result;
  });
  ipcMain.handle('terminal:create', (_event, params) => ({
    terminalId: terminalManager.createTerminal(params),
  }));
  ipcMain.handle('terminal:output', (_event, params: { terminalId: string }) =>
    terminalManager.getOutput(params.terminalId)
  );
  ipcMain.handle('terminal:wait_for_exit', (_event, params: { terminalId: string }) =>
    terminalManager.waitForExit(params.terminalId)
  );
  ipcMain.handle('terminal:kill', (_event, params: { terminalId: string }) =>
    terminalManager.kill(params.terminalId)
  );
  ipcMain.handle('terminal:release', (_event, params: { terminalId: string }) =>
    terminalManager.release(params.terminalId)
  );
  registerAcpFileSystemHandlers();
  registerConfigHandlers();
  registerVec0Handlers();
  registerVoiceHandlers();
  registerPermissionLedgerHandlers();
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
  appendFileSync(join(app.getPath('userData'), 'crash.log'), `[${new Date().toISOString()}] uncaughtException: ${error.stack || error}\n`);
  process.exit(1);
});
process.on('unhandledRejection', (reason) => {
  console.error('[maenifold] Unhandled rejection:', reason);
});
app.on('before-quit', () => { isQuitting = true; });
app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
app.on('activate', async () => { if (BrowserWindow.getAllWindows().length === 0) await createWindow(); else mainWindow?.show(); });
app.whenReady().then(async () => {
  registerIpcHandlers();
  registerSandboxProtocol();
  await ensureServices();
  await createWindow();
});
