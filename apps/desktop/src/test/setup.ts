// Global test setup — safe browser mocks for jsdom.
import { vi } from 'vitest';
import { ref } from 'vue';

function createStorageMock(): Storage {
  const values = new Map<string, string>();

  return {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key: string) {
      return values.has(key) ? values.get(key)! : null;
    },
    key(index: number) {
      return Array.from(values.keys())[index] ?? null;
    },
    removeItem(key: string) {
      values.delete(key);
    },
    setItem(key: string, value: string) {
      values.set(key, value);
    },
  };
}

Object.defineProperty(window, 'localStorage', {
  value: createStorageMock(),
  configurable: true,
});

// jsdom polyfill: reka-ui (and other libs) use ResizeObserver at runtime.
// jsdom does not provide one, so we install a no-op shim for tests.
if (typeof globalThis.ResizeObserver === 'undefined') {
  class ResizeObserverShim {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  globalThis.ResizeObserver = ResizeObserverShim;
}

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
