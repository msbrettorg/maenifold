// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { NotificationSink } from '@/lib/notification-sink';

const mockSink: NotificationSink = {
  success: vi.fn(),
  info: vi.fn(),
  warning: vi.fn(),
  error: vi.fn(),
  critical: vi.fn(),
};

let resetNotificationSink: (() => void) | null = null;

describe('config store error sanitization', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    setActivePinia(createPinia());
    Reflect.deleteProperty(window, 'electronAPI');
    const notificationSink = await import('@/lib/notification-sink');
    notificationSink.setNotificationSink(mockSink);
    resetNotificationSink = notificationSink.resetNotificationSink;
  });

  afterEach(() => {
    resetNotificationSink?.();
    resetNotificationSink = null;
  });

  it('redacts sensitive paths before surfacing loadConfig errors', async () => {
    const raw = 'EACCES /Users/brett/secret/file.txt tenantId: a1b2c3d4-e5f6-1234-9abc-def012345678';
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        configRead: vi.fn().mockRejectedValueOnce(new Error(raw)),
        configWrite: vi.fn(),
        configPath: vi.fn(),
        onConfigChanged: vi.fn().mockReturnValue(vi.fn()),
      },
    });
    const { useConfigStore } = await import('../config');

    const store = useConfigStore();
    await store.loadConfig();

    expect(store.error).toContain('<redacted>');
    expect(store.error).not.toContain('/Users/brett/secret/file.txt');
    expect(store.error).not.toContain('a1b2c3d4-e5f6-1234-9abc-def012345678');
    expect(mockSink.error).toHaveBeenCalledWith(expect.stringContaining('<redacted>'));
  });
});
