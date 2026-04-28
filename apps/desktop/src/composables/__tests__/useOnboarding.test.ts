// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { PREFS_STORE_PATH } from '../../lib/constants';
import { storageKey } from '../../lib/local-storage';
import { useOnboarding } from '../useOnboarding';

const ONBOARDING_KEY = 'onboarding';
const CURRENT_VERSION = 1;

function fullKey(): string {
  return storageKey(PREFS_STORE_PATH, ONBOARDING_KEY);
}

function stubConfigRead(result: { content: string; exists: boolean }): void {
  vi.stubGlobal('electronAPI', {
    configRead: vi.fn().mockResolvedValue({
      path: '/Users/example/maenifold/maenifold.json',
      ...result,
    }),
  });
}

describe('useOnboarding', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    window.localStorage.clear();
  });

  it('checkFirstRun returns true when no flag exists', () => {
    const onboarding = useOnboarding();
    expect(onboarding.checkFirstRun()).toBe(true);
    expect(onboarding.isFirstRun.value).toBe(true);
  });

  it('checkFirstRun returns false when flag exists with current version', () => {
    window.localStorage.setItem(
      fullKey(),
      JSON.stringify({ completed: true, version: CURRENT_VERSION, completedAt: '2025-01-01T00:00:00.000Z' }),
    );
    const onboarding = useOnboarding();
    expect(onboarding.checkFirstRun()).toBe(false);
    expect(onboarding.isFirstRun.value).toBe(false);
  });

  it('checkFirstRun returns true when flag version is outdated', () => {
    window.localStorage.setItem(
      fullKey(),
      JSON.stringify({ completed: true, version: 0 }),
    );
    const onboarding = useOnboarding();
    expect(onboarding.checkFirstRun()).toBe(true);
    expect(onboarding.isFirstRun.value).toBe(true);
  });

  it('advance increments step and highWaterStep', () => {
    const onboarding = useOnboarding();
    onboarding.advance();
    expect(onboarding.step.value).toBe(1);
    expect(onboarding.highWaterStep.value).toBe(1);
    expect(onboarding.currentStepId.value).toBe('daemon');
  });

  it('advance tracks highWaterStep correctly across back-navigation', () => {
    const onboarding = useOnboarding();
    onboarding.advance(); // step=1, hwm=1
    onboarding.advance(); // step=2, hwm=2
    onboarding.goBack(); // step=1, hwm=2
    expect(onboarding.step.value).toBe(1);
    expect(onboarding.highWaterStep.value).toBe(2);
    onboarding.advance(); // step=2, hwm=2
    expect(onboarding.step.value).toBe(2);
    expect(onboarding.highWaterStep.value).toBe(2);
  });

  it('goToStep allows visited steps only', () => {
    const onboarding = useOnboarding();
    onboarding.advance();
    onboarding.advance(); // step=2, hwm=2
    onboarding.goToStep(1);
    expect(onboarding.step.value).toBe(1);
    onboarding.goToStep(3);
    expect(onboarding.step.value).toBe(1);
  });

  it('goToStep rejects out-of-range values', () => {
    const onboarding = useOnboarding();
    onboarding.advance(); // step=1, hwm=1
    onboarding.goToStep(-1);
    expect(onboarding.step.value).toBe(1);
    onboarding.goToStep(999);
    expect(onboarding.step.value).toBe(1);
  });

  it('goBack decrements step', () => {
    const onboarding = useOnboarding();
    onboarding.advance();
    onboarding.goBack();
    expect(onboarding.step.value).toBe(0);
  });

  it('goBack is no-op at step 0', () => {
    const onboarding = useOnboarding();
    onboarding.goBack();
    expect(onboarding.step.value).toBe(0);
  });

  it('markComplete persists flag and clears isFirstRun', () => {
    const onboarding = useOnboarding();
    onboarding.markComplete();
    expect(onboarding.isFirstRun.value).toBe(false);
    const raw = window.localStorage.getItem(fullKey());
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw as string);
    expect(parsed.completed).toBe(true);
    expect(parsed.version).toBe(CURRENT_VERSION);
    expect(typeof parsed.completedAt).toBe('string');
  });

  it('round-trip: checkFirstRun -> markComplete -> checkFirstRun returns false', () => {
    const onboarding = useOnboarding();
    expect(onboarding.checkFirstRun()).toBe(true);
    onboarding.markComplete();
    const next = useOnboarding();
    expect(next.checkFirstRun()).toBe(false);
  });

  it('checkFirstRunWithConfig returns false and persists flag when existing agents map is non-empty', async () => {
    stubConfigRead({
      exists: true,
      content: JSON.stringify({ agents: { claude: { command: 'claude' } } }),
    });

    const onboarding = useOnboarding();
    await expect(onboarding.checkFirstRunWithConfig()).resolves.toBe(false);
    expect(onboarding.isFirstRun.value).toBe(false);

    const raw = window.localStorage.getItem(fullKey());
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw as string);
    expect(parsed.completed).toBe(true);
    expect(parsed.version).toBe(CURRENT_VERSION);
  });

  it('checkFirstRunWithConfig returns true when configRead reports no config', async () => {
    stubConfigRead({ exists: false, content: '' });

    const onboarding = useOnboarding();
    await expect(onboarding.checkFirstRunWithConfig()).resolves.toBe(true);
    expect(onboarding.isFirstRun.value).toBe(true);
    expect(window.localStorage.getItem(fullKey())).toBeNull();
  });

  it('checkFirstRunWithConfig returns false when existing config has empty agents', async () => {
    stubConfigRead({
      exists: true,
      content: JSON.stringify({ agents: {} }),
    });

    const onboarding = useOnboarding();
    await expect(onboarding.checkFirstRunWithConfig()).resolves.toBe(false);
    expect(onboarding.isFirstRun.value).toBe(false);
    expect(window.localStorage.getItem(fullKey())).not.toBeNull();
  });

  it('checkFirstRunWithConfig returns true when config content is empty whitespace', async () => {
    stubConfigRead({ exists: true, content: '   ' });

    const onboarding = useOnboarding();
    await expect(onboarding.checkFirstRunWithConfig()).resolves.toBe(true);
    expect(onboarding.isFirstRun.value).toBe(true);
    expect(window.localStorage.getItem(fullKey())).toBeNull();
  });

  it('checkFirstRunWithConfig keeps isFirstRun null until configRead resolves', async () => {
    let resolveConfig!: (value: { exists: boolean; content: string; path: string }) => void;
    const configReadPromise = new Promise<{ exists: boolean; content: string; path: string }>((resolve) => {
      resolveConfig = resolve;
    });
    vi.stubGlobal('electronAPI', {
      configRead: vi.fn(() => configReadPromise),
    });

    const onboarding = useOnboarding();
    const checkPromise = onboarding.checkFirstRunWithConfig();

    await Promise.resolve();

    expect(onboarding.isFirstRun.value).toBe(null);

    resolveConfig({
      exists: true,
      content: JSON.stringify({ agents: { foo: {} } }),
      path: '/x/maenifold.json',
    });
    await checkPromise;

    expect(onboarding.isFirstRun.value).toBe(false);
  });

  it('checkFirstRunWithConfig keeps isFirstRun null until configRead resolves (no config)', async () => {
    let resolveConfig!: (value: { exists: boolean; content: string; path: string }) => void;
    const configReadPromise = new Promise<{ exists: boolean; content: string; path: string }>((resolve) => {
      resolveConfig = resolve;
    });
    vi.stubGlobal('electronAPI', {
      configRead: vi.fn(() => configReadPromise),
    });

    const onboarding = useOnboarding();
    const checkPromise = onboarding.checkFirstRunWithConfig();

    await Promise.resolve();

    expect(onboarding.isFirstRun.value).toBe(null);

    resolveConfig({ exists: false, content: '', path: '/x/maenifold.json' });
    await checkPromise;

    expect(onboarding.isFirstRun.value).toBe(true);
  });

  it('checkFirstRunWithConfig returns true when configRead throws', async () => {
    vi.stubGlobal('electronAPI', {
      configRead: vi.fn().mockRejectedValue(new Error('config read failed')),
    });

    const onboarding = useOnboarding();
    await expect(onboarding.checkFirstRunWithConfig()).resolves.toBe(true);
    expect(onboarding.isFirstRun.value).toBe(true);
    expect(window.localStorage.getItem(fullKey())).toBeNull();
  });
});
