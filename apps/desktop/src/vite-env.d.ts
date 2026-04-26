/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_MCP_BASE_URL?: string;
  readonly VITE_DAEMON_URL?: string;
  readonly VITE_APP_VERSION?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

type ElectronVoiceCheckResult = {
  installed: boolean;
  path: string | null;
  running: boolean;
  installHint: string;
};

type ElectronVoiceLaunchResult = {
  ok: boolean;
  error?: string;
  alreadyRunning?: boolean;
  pid?: number;
};

type ElectronApi = {
  openDirectory?: () => Promise<string | null>;
  getAppVersion?: () => Promise<string>;
  getAuthToken?: () => Promise<string>;
  checkEnvVarPresence?: (names: string[]) => Promise<Record<string, boolean>>;
  vec0Status?: () => Promise<Vec0Status>;
  vec0Download?: () => Promise<Vec0Status>;
  voiceCheck?: () => Promise<ElectronVoiceCheckResult>;
  voiceLaunch?: () => Promise<ElectronVoiceLaunchResult>;
};

type Vec0DownloadProgress = {
  current: number;
  total: number;
  filename?: string;
};

type Vec0Status = {
  platform: string;
  arch: string;
  supported: boolean;
  installed: boolean;
  fileSizeBytes: number;
  assetPattern: string | null;
  targetPath: string | null;
  downloadProgress: Vec0DownloadProgress | null;
};

interface Window {
  electronAPI?: ElectronApi;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module 'cytoscape-cose-bilkent' {
  const ext: cytoscape.Ext;
  export default ext;
}
