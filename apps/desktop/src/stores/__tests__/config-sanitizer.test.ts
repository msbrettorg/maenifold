// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { toastError } = vi.hoisted(() => ({ toastError: vi.fn() }));

vi.mock('@/composables/useToast', () => ({ toastError }));

describe('config store error sanitization', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    setActivePinia(createPinia());
    Reflect.deleteProperty(window, 'electronAPI');
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
    expect(toastError).toHaveBeenCalledWith(expect.stringContaining('<redacted>'));
  });
});
