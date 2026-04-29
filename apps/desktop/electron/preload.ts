import { contextBridge, ipcRenderer } from 'electron';

type Vec0Status = {
  platform: string;
  arch: string;
  supported: boolean;
  installed: boolean;
  fileSizeBytes: number;
  assetPattern: string | null;
  targetPath: string | null;
  downloadProgress: {
    current: number;
    total: number;
    filename?: string;
  } | null;
};

type PermissionLedgerPayload = {
  request: unknown;
  context: unknown;
};

const electronAPI = {
  openDirectory: () => ipcRenderer.invoke('dialog:open-directory') as Promise<string | null>,
  getAppVersion: () => ipcRenderer.invoke('app:get-version') as Promise<string>,
  readTextFile: (params: { path: string; line?: number; limit?: number }) =>
    ipcRenderer.invoke('acp:read-text-file', params) as Promise<{ content: string }>,
  writeTextFile: (params: { path: string; content: string }) =>
    ipcRenderer.invoke('acp:write-text-file', params) as Promise<Record<string, never>>,
  checkEnvVarPresence: (names: string[]) =>
    ipcRenderer.invoke('check-env-vars', names) as Promise<Record<string, boolean>>,
  configRead: () =>
    ipcRenderer.invoke('config:read') as Promise<{ path: string; content: string; exists: boolean }>,
  configWrite: (content: string) =>
    ipcRenderer.invoke('config:write', content) as Promise<{ path: string }>,
  configPath: () => ipcRenderer.invoke('config:path') as Promise<string>,
  onConfigChanged: (handler: () => void) => {
    const listener = () => handler();
    ipcRenderer.on('config:changed', listener);
    return () => ipcRenderer.removeListener('config:changed', listener);
  },
  vec0Status: () => ipcRenderer.invoke('vec0:status') as Promise<Vec0Status>,
  vec0Download: () => ipcRenderer.invoke('vec0:download') as Promise<Vec0Status>,
  voiceCheck: () => ipcRenderer.invoke('voice:check') as Promise<{
    installed: boolean;
    path: string | null;
    running: boolean;
  }>,
  voiceLaunch: () => ipcRenderer.invoke('voice:launch') as Promise<{
    ok: boolean;
    error?: string;
    alreadyRunning?: boolean;
    pid?: number;
  }>,
  permissionTryAutoDecide: (payload: PermissionLedgerPayload) =>
    ipcRenderer.invoke('permission:try-auto-decide', payload) as Promise<unknown>,
  permissionRecordDecision: (payload: PermissionLedgerPayload) =>
    ipcRenderer.invoke('permission:record-decision', payload) as Promise<unknown>,
  permissionListDecisions: () => ipcRenderer.invoke('permission:list-decisions') as Promise<unknown[]>,
  permissionListAuditEvents: () =>
    ipcRenderer.invoke('permission:list-audit-events') as Promise<unknown[]>,
  permissionRevokeDecision: (id: number) =>
    ipcRenderer.invoke('permission:revoke-decision', id) as Promise<void>,
  permissionResetDecisions: () => ipcRenderer.invoke('permission:reset-decisions') as Promise<void>,
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);
