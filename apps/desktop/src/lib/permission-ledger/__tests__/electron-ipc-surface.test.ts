import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('electron permission IPC surface contract', () => {
  it('preload and renderer types expose permissionEvaluateRequest instead of permissionTryAutoDecide', async () => {
    const [preload, viteEnv] = await Promise.all([
      readFile(join(process.cwd(), 'electron/preload.ts'), 'utf8'),
      readFile(join(process.cwd(), 'src/vite-env.d.ts'), 'utf8'),
    ]);

    expect(preload).not.toContain('permissionTryAutoDecide');
    expect(preload).toContain('permissionEvaluateRequest');
    expect(viteEnv).not.toContain('permissionTryAutoDecide');
    expect(viteEnv).toContain('permissionEvaluateRequest');
  });

  it('renderer preload surface has evaluate-request function and no try-auto-decide function', () => {
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        permissionEvaluateRequest: () => Promise.resolve({ autoDecided: false }),
      },
    });

    expect(
      (window.electronAPI as unknown as Record<string, unknown>)?.permissionTryAutoDecide
    ).toBeUndefined();
    expect(
      typeof (window.electronAPI as unknown as { permissionEvaluateRequest?: unknown })
        .permissionEvaluateRequest
    ).toBe('function');
  });
});
