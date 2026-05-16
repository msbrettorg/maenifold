// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { PREFS_STORE_PATH } from '../../lib/constants';
import { storageKey } from '../../lib/local-storage';
import { useOnboarding, type OnboardingStepId } from '../useOnboarding';

const ONBOARDING_KEY = 'onboarding';
const ONBOARDING_PROGRESS_KEY = 'onboarding-progress';
const CURRENT_VERSION = 1;

type OnboardingProgressApi = ReturnType<typeof useOnboarding> & {
  restoreProgress: () => void; markStepDone: (stepId: OnboardingStepId) => void;
  isStepDone: (stepId: OnboardingStepId) => boolean; resumeAtFirstIncomplete: () => void;
};

function fullKey(): string {
  return storageKey(PREFS_STORE_PATH, ONBOARDING_KEY);
}

function progressKey(): string { return storageKey(PREFS_STORE_PATH, ONBOARDING_PROGRESS_KEY); }

function progressState(): { step?: number; highWaterStep?: number; markers?: Record<string, boolean> } { return JSON.parse(window.localStorage.getItem(progressKey()) as string); }

function advanceTo(onboarding: ReturnType<typeof useOnboarding>, target: number): void { Array.from({ length: target }).forEach(() => onboarding.advance()); }

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

  describe('onboarding idempotency (T-317.1)', () => {
    it('persists step progress on advance and restores it in a new instance', () => {
      const onboarding = useOnboarding() as OnboardingProgressApi;
      advanceTo(onboarding, 2);
      expect(progressState()).toMatchObject({ step: 2, highWaterStep: 2 });
      const restarted = useOnboarding() as OnboardingProgressApi;
      restarted.restoreProgress();
      expect(restarted.step.value).toBe(2); expect(restarted.currentStepId.value).toBe('agent');
    });
    it('resumes on step 3 model after force-quit mid-step 3', () => {
      const onboarding = useOnboarding() as OnboardingProgressApi;
      advanceTo(onboarding, 3);
      const restarted = useOnboarding() as OnboardingProgressApi;
      restarted.restoreProgress();
      expect(restarted.step.value).toBe(3); expect(restarted.currentStepId.value).toBe('model');
    });
    it('persists highWaterStep after back navigation and restart', () => {
      const onboarding = useOnboarding() as OnboardingProgressApi;
      advanceTo(onboarding, 3);
      onboarding.goBack(); onboarding.goBack();
      const restarted = useOnboarding() as OnboardingProgressApi;
      restarted.restoreProgress();
      expect(restarted.step.value).toBe(1); expect(restarted.highWaterStep.value).toBe(3);
    });
    it('persists per-step progress markers for completed onboarding steps', () => {
      const onboarding = useOnboarding() as OnboardingProgressApi;
      ['daemon', 'agent', 'model'].forEach((stepId) => onboarding.markStepDone(stepId as OnboardingStepId));
      expect(progressState().markers).toMatchObject({
        'daemon-checked': true,
        'agent-picked': true,
        'model-installed': true,
      });
      expect(onboarding.isStepDone('model')).toBe(true);
    });
    it('resumes at the first incomplete step from persisted markers', () => {
      const onboarding = useOnboarding() as OnboardingProgressApi;
      ['welcome', 'daemon', 'agent'].forEach((stepId) => onboarding.markStepDone(stepId as OnboardingStepId));
      const restarted = useOnboarding() as OnboardingProgressApi;
      restarted.resumeAtFirstIncomplete();
      expect(restarted.step.value).toBe(3); expect(restarted.currentStepId.value).toBe('model');
    });
    it('re-entering a completed step is idempotent and keeps detected state', () => {
      const onboarding = useOnboarding() as OnboardingProgressApi;
      onboarding.advance();
      onboarding.markStepDone('daemon');
      onboarding.advance();
      onboarding.goBack();
      expect(onboarding.step.value).toBe(1); expect(onboarding.currentStepId.value).toBe('daemon');
      expect(onboarding.isStepDone('daemon')).toBe(true);
      expect(progressState().markers?.['daemon-checked']).toBe(true);
    });
  });
});
