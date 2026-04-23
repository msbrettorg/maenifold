import { contextBridge, ipcRenderer } from 'electron';

const electronAPI = {
  openDirectory: () => ipcRenderer.invoke('dialog:open-directory') as Promise<string | null>,
  getAppVersion: () => ipcRenderer.invoke('app:get-version') as Promise<string>,
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);
