import { contextBridge, ipcRenderer } from 'electron';

const electronAPI = {
  openDirectory: () => ipcRenderer.invoke('dialog:open-directory') as Promise<string | null>,
  openFile: () => ipcRenderer.invoke('dialog:open-file') as Promise<string | null>,
  readBinaryFile: (filePath: string) =>
    ipcRenderer.invoke('file:read-binary', filePath) as Promise<Uint8Array>,
  getAppVersion: () => ipcRenderer.invoke('app:get-version') as Promise<string>,
  restartDaemon: () => ipcRenderer.invoke('daemon:restart') as Promise<void>,
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);
