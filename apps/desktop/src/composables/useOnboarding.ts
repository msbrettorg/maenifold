import { computed, toValue, type ComputedRef, ref, type MaybeRefOrGetter, type Ref } from 'vue';

import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';

export const ONBOARDING_STEPS = ['welcome', 'daemon', 'agent', 'model', 'vec0', 'done'] as const;
export const DEFAULT_ONBOARDING_STEPS = ['welcome', 'daemon', 'agent', 'model', 'done'] as const;
export type OnboardingStepId = (typeof ONBOARDING_STEPS)[number];

const ONBOARDING_KEY = 'onboarding';
const PROGRESS_KEY = 'onboarding-progress';
const CURRENT_VERSION = 1;

const STEP_MARKER_MAP: Record<OnboardingStepId, string> = {
  welcome: 'welcome-seen',
  daemon: 'daemon-checked',
  agent: 'agent-picked',
  model: 'model-installed',
  vec0: 'vec0-configured',
  done: 'onboarding-done',
};

interface OnboardingState {
  completed: boolean;
  completedAt?: string;
  version: number;
}

interface ProgressState {
  step: number;
  highWaterStep: number;
  markers: Record<string, boolean>;
}

export interface UseOnboardingReturn {
  isFirstRun: Ref<boolean | null>;
  step: Ref<number>;
  highWaterStep: Ref<number>;
  currentStepId: ComputedRef<OnboardingStepId>;
  canGoBack: ComputedRef<boolean>;
  canGoForward: ComputedRef<boolean>;
  steps: ComputedRef<readonly OnboardingStepId[]>;
  checkFirstRun: () => boolean;
  checkFirstRunWithConfig: () => Promise<boolean>;
  advance: () => void;
  goToStep: (target: number) => void;
  goBack: () => void;
  restoreProgress: () => void;
  markStepDone: (stepId: OnboardingStepId) => void;
  isStepDone: (stepId: OnboardingStepId) => boolean;
  resumeAtFirstIncomplete: () => void;
  markComplete: () => void;
}

function readOnboardingState(): OnboardingState | null {
  return readStorageValue<OnboardingState>(PREFS_STORE_PATH, ONBOARDING_KEY);
}

function readProgressState(): ProgressState | null {
  const state = readStorageValue<Partial<ProgressState>>(PREFS_STORE_PATH, PROGRESS_KEY);
  if (!isRecord(state)) {
    return null;
  }
  const markers = isRecord(state.markers) ? state.markers : {};
  return {
    step: typeof state.step === 'number' ? state.step : 0,
    highWaterStep: typeof state.highWaterStep === 'number' ? state.highWaterStep : 0,
    markers: Object.fromEntries(
      Object.entries(markers).filter((entry): entry is [string, boolean] => typeof entry[1] === 'boolean'),
    ),
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export async function hasExistingMaenifoldConfig(): Promise<boolean> {
  try {
    const result = await window.electronAPI?.configRead?.();
    if (!result?.exists || result.content.trim().length === 0) {
      return false;
    }
    const parsed: unknown = JSON.parse(result.content);
    return isRecord(parsed);
  } catch {
    return false;
  }
}

export function useOnboarding(
  stepSource: MaybeRefOrGetter<readonly OnboardingStepId[]> = DEFAULT_ONBOARDING_STEPS,
): UseOnboardingReturn {
  const isFirstRun = ref<boolean | null>(null);
  const step = ref<number>(0);
  const highWaterStep = ref<number>(0);

  const steps = computed<readonly OnboardingStepId[]>(() => {
    const resolved = toValue(stepSource);
    return resolved.length > 0 ? resolved : DEFAULT_ONBOARDING_STEPS;
  });
  const lastIndex = computed<number>(() => steps.value.length - 1);

  const currentStepId = computed<OnboardingStepId>(() => {
    const idx = Math.min(Math.max(step.value, 0), lastIndex.value);
    return steps.value[idx] ?? 'done';
  });

  const canGoBack = computed<boolean>(() => step.value > 0);
  const canGoForward = computed<boolean>(() => step.value < highWaterStep.value);

  function clampStep(value: number): number {
    return Math.min(Math.max(value, 0), lastIndex.value);
  }

  function currentMarkers(): Record<string, boolean> {
    return readProgressState()?.markers ?? {};
  }

  function persistProgress(markers: Record<string, boolean> = currentMarkers()): void {
    const state: ProgressState = {
      step: clampStep(step.value),
      highWaterStep: clampStep(highWaterStep.value),
      markers,
    };
    writeStorageValue(PREFS_STORE_PATH, PROGRESS_KEY, state);
  }

  function restoreProgress(): void {
    const state = readProgressState();
    if (!state) {
      return;
    }
    highWaterStep.value = clampStep(Math.max(state.step, state.highWaterStep));
    step.value = clampStep(state.step);
    persistProgress();
  }

  function markStepDone(stepId: OnboardingStepId): void {
    const markers = currentMarkers();
    markers[STEP_MARKER_MAP[stepId]] = true;
    persistProgress(markers);
  }

  function isStepDone(stepId: OnboardingStepId): boolean {
    return currentMarkers()[STEP_MARKER_MAP[stepId]] === true;
  }

  function resumeAtFirstIncomplete(): void {
    const markers = currentMarkers();
    const firstIncomplete = steps.value.findIndex((stepId) => markers[STEP_MARKER_MAP[stepId]] !== true);
    const nextStep = firstIncomplete === -1 ? lastIndex.value : firstIncomplete;
    step.value = nextStep;
    if (nextStep > highWaterStep.value) {
      highWaterStep.value = nextStep;
    }
    persistProgress(markers);
  }

  function checkFirstRun(): boolean {
    const state = readOnboardingState();
    if (!state || !state.completed || state.version < CURRENT_VERSION) {
      isFirstRun.value = true;
      return true;
    }
    isFirstRun.value = false;
    return false;
  }

  async function checkFirstRunWithConfig(): Promise<boolean> {
    const localState = readOnboardingState();
    const localSaysReturning =
      !!localState && localState.completed && localState.version >= CURRENT_VERSION;

    if (localSaysReturning) {
      isFirstRun.value = false;
      return false;
    }
    if (await hasExistingMaenifoldConfig()) {
      markComplete();
      return false;
    }
    isFirstRun.value = true;
    return true;
  }

  function advance(): void {
    const nextStep = Math.min(step.value + 1, lastIndex.value);
    if (nextStep === step.value) {
      return;
    }
    if (nextStep > highWaterStep.value) {
      highWaterStep.value = nextStep;
    }
    step.value = nextStep;
    persistProgress();
  }

  function goToStep(target: number): void {
    if (!Number.isInteger(target)) {
      return;
    }
    if (target < 0 || target > highWaterStep.value) {
      return;
    }
    step.value = target;
    persistProgress();
  }

  function goBack(): void {
    if (step.value <= 0) {
      return;
    }
    step.value -= 1;
    persistProgress();
  }

  function markComplete(): void {
    const state: OnboardingState = {
      completed: true,
      completedAt: new Date().toISOString(),
      version: CURRENT_VERSION,
    };
    writeStorageValue(PREFS_STORE_PATH, ONBOARDING_KEY, state);
    isFirstRun.value = false;
  }

  return {
    isFirstRun,
    step,
    highWaterStep,
    currentStepId,
    canGoBack,
    canGoForward,
    steps,
    checkFirstRun,
    checkFirstRunWithConfig,
    advance,
    goToStep,
    goBack,
    restoreProgress,
    markStepDone,
    isStepDone,
    resumeAtFirstIncomplete,
    markComplete,
  };
}
