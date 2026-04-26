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

const electronAPI = {
  openDirectory: () => ipcRenderer.invoke('dialog:open-directory') as Promise<string | null>,
  getAppVersion: () => ipcRenderer.invoke('app:get-version') as Promise<string>,
  getAuthToken: () => ipcRenderer.invoke('get-auth-token') as Promise<string>,
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
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);
