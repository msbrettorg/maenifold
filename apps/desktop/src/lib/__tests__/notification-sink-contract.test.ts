import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  createNoopNotificationSink,
  getNotificationSink,
  resetNotificationSink,
  setNotificationSink,
  type NotificationSink,
} from '@/lib/notification-sink';

const storesRoot = resolve(process.cwd(), 'src/stores');
const toastImportNeedles = [
  "from '@/composables/useToast'",
  'from "../composables/useToast"',
  'from "../../composables/useToast"',
];

function listProductionStoreFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = join(dir, entry.name);
    if (fullPath.includes('__tests__')) return [];
    if (entry.isDirectory()) return listProductionStoreFiles(fullPath);
    return entry.isFile() && fullPath.endsWith('.ts') ? [fullPath] : [];
  });
}

const productionStoreFiles = listProductionStoreFiles(storesRoot);

afterEach(() => {
  resetNotificationSink();
});

describe('NotificationSink contract', () => {
  it('no production store imports @/composables/useToast', () => {
    const offenders = productionStoreFiles.filter((file) => {
      const source = readFileSync(file, 'utf8');
      return toastImportNeedles.some((needle) => source.includes(needle));
    });

    expect(
      offenders,
      `Stores must use getNotificationSink() instead of useToast:\n${offenders
        .map((file) => relative(storesRoot, file))
        .join('\n')}`,
    ).toEqual([]);
  });

  it('every production store using a sink uses getNotificationSink', () => {
    const offenders = productionStoreFiles.filter((file) => {
      const source = readFileSync(file, 'utf8');
      return (
        (source.includes('Sink(') || source.includes('notification-sink')) &&
        !source.includes("from '@/lib/notification-sink'")
      );
    });

    expect(
      offenders,
      `Stores using a notification sink must import @/lib/notification-sink:\n${offenders
        .map((file) => relative(storesRoot, file))
        .join('\n')}`,
    ).toEqual([]);
  });

  it('NotificationSink covers all 5 severity levels', () => {
    const noopSink = createNoopNotificationSink();
    const customError = vi.fn();
    const customSink: NotificationSink = {
      success: vi.fn(),
      info: vi.fn(),
      warning: vi.fn(),
      error: customError,
      critical: vi.fn(),
    };

    expect(noopSink).toEqual(
      expect.objectContaining({
        success: expect.any(Function),
        info: expect.any(Function),
        warning: expect.any(Function),
        error: expect.any(Function),
        critical: expect.any(Function),
      }),
    );

    setNotificationSink(customSink);
    getNotificationSink().error('m');
    expect(customError).toHaveBeenCalledWith('m');

    resetNotificationSink();
    getNotificationSink().error('after reset');
    expect(customError).toHaveBeenCalledTimes(1);
  });

  it('setNotificationSink + resetNotificationSink round-trip', () => {
    const customSink: NotificationSink = {
      success: vi.fn(),
      info: vi.fn(),
      warning: vi.fn(),
      error: vi.fn(),
      critical: vi.fn(),
    };

    setNotificationSink(customSink);
    expect(getNotificationSink().error).toBe(customSink.error);

    resetNotificationSink();
    expect(getNotificationSink().error).not.toBe(customSink.error);
  });
});
