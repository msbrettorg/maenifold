// Global test setup — mocks Tauri IPC layer so tests run in plain jsdom.
import { vi } from 'vitest';
import { ref } from 'vue';

// ── Mock Tauri packages ───────────────────────────────────────────────
// These packages require the Tauri runtime. We mock them at the npm
// module level so every source file that imports them gets safe stubs.

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@tauri-apps/plugin-fs', () => ({
  mkdir: vi.fn().mockResolvedValue(undefined),
  readTextFile: vi.fn().mockResolvedValue(''),
  writeTextFile: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/path', () => ({
  dirname: vi.fn().mockResolvedValue('/mock'),
}));

vi.mock('@tauri-apps/plugin-store', () => ({
  load: vi.fn().mockResolvedValue({
    get: vi.fn().mockResolvedValue(null),
    set: vi.fn().mockResolvedValue(undefined),
    save: vi.fn().mockResolvedValue(undefined),
  }),
  Store: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.1.0'),
}));

// ── Mock local Tauri wrapper ──────────────────────────────────────────
// src/lib/tauri.ts wraps @tauri-apps invoke/listen calls.
// Mocking it prevents any transitive Tauri IPC at the application layer.

vi.mock('../lib/tauri', () => ({
  getConfig: vi.fn().mockResolvedValue({ agents: {} }),
  reloadConfig: vi.fn().mockResolvedValue({ agents: {} }),
  getConfigPath: vi.fn().mockResolvedValue('/mock/config/path'),
  spawnAgent: vi.fn().mockResolvedValue({ id: 'mock-agent-id', name: 'mock-agent' }),
  sendToAgent: vi.fn().mockResolvedValue(undefined),
  killAgent: vi.fn().mockResolvedValue(undefined),
  listRunningAgents: vi.fn().mockResolvedValue([]),
  addAgent: vi.fn().mockResolvedValue({ agents: {} }),
  removeAgent: vi.fn().mockResolvedValue({ agents: {} }),
  updateAgent: vi.fn().mockResolvedValue({ agents: {} }),
  onAgentMessage: vi.fn().mockResolvedValue(() => {}),
  onAgentClosed: vi.fn().mockResolvedValue(() => {}),
  onConfigChanged: vi.fn().mockResolvedValue(() => {}),
  onAgentStderr: vi.fn().mockResolvedValue(() => {}),
  getMachineId: vi.fn().mockResolvedValue('mock-machine-id'),
}));

// ── MockAcpClientBridge ───────────────────────────────────────────────
// Drop-in test double for AcpClientBridge that never touches IPC.

export class MockAcpClientBridge {
  agentId: string;
  onSessionUpdate: ((notification: unknown) => void) | null = null;
  pendingPermissionRequests = ref<unknown[]>([]);

  constructor(agentId = 'mock-agent') {
    this.agentId = agentId;
  }

  connect = vi.fn().mockResolvedValue(undefined);
  disconnect = vi.fn().mockResolvedValue(undefined);
  initialize = vi.fn().mockResolvedValue({});
  newSession = vi.fn().mockResolvedValue({ sessionId: 'mock-session' });
  loadSession = vi.fn().mockResolvedValue({});
  listSessions = vi.fn().mockResolvedValue({ sessions: [] });
  prompt = vi.fn().mockResolvedValue({ stopReason: 'end_turn' });
  cancel = vi.fn().mockResolvedValue(undefined);
  setMode = vi.fn().mockResolvedValue(undefined);
  supportsAgentMethod = vi.fn().mockReturnValue(false);
  unstable_setSessionModel = vi.fn().mockResolvedValue(undefined);
  unstable_setSessionConfigOption = vi.fn().mockResolvedValue({ configOptions: [] });
  authenticate = vi.fn().mockResolvedValue({});
  resolvePermission = vi.fn();
  cancelPermission = vi.fn();
  requestPermission = vi.fn();
  sessionUpdate = vi.fn();
  writeTextFile = vi.fn().mockResolvedValue(null);
  readTextFile = vi.fn().mockResolvedValue({ content: '' });
}

/** Factory shorthand for creating a fresh MockAcpClientBridge. */
export function createMockBridge(agentId?: string): MockAcpClientBridge {
  return new MockAcpClientBridge(agentId);
}
