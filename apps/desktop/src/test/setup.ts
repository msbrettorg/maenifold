// Global test setup — safe browser mocks for jsdom.
import { vi } from 'vitest';
import { ref } from 'vue';

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
  watch: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@tauri-apps/api/path', () => ({
  dirname: vi.fn().mockResolvedValue('/mock'),
}));

export class MockAcpClientBridge {
  agentId: string;
  onSessionUpdate: ((notification: unknown) => void) | null = null;
  onStderr: ((line: string) => void) | null = null;
  pendingPermissionRequests = ref<unknown[]>([]);
  private transportClosed = false;
  private transportCloseReason: string | null = null;

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
  supportsAgentMethod = vi.fn().mockReturnValue(false);
  supportsSessionClose = vi.fn().mockReturnValue(false);
  unstable_setSessionConfigOption = vi.fn().mockResolvedValue({ configOptions: [] });
  closeSession = vi.fn().mockResolvedValue(undefined);
  authenticate = vi.fn().mockResolvedValue({});
  resolvePermission = vi.fn();
  cancelPermission = vi.fn();
  requestPermission = vi.fn();
  sessionUpdate = vi.fn();
  writeTextFile = vi.fn().mockResolvedValue(null);
  readTextFile = vi.fn().mockResolvedValue({ content: '' });
  isTransportClosed = vi.fn(() => this.transportClosed);
  getTransportCloseReason = vi.fn(() => this.transportCloseReason);

  markTransportClosed(reason = 'mock close') {
    this.transportClosed = true;
    this.transportCloseReason = reason;
  }
}

/** Factory shorthand for creating a fresh MockAcpClientBridge. */
export function createMockBridge(agentId?: string): MockAcpClientBridge {
  return new MockAcpClientBridge(agentId);
}
