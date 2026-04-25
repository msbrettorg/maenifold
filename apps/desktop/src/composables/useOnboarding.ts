import { computed, toValue, type ComputedRef, ref, type MaybeRefOrGetter, type Ref } from 'vue';

import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';

export const ONBOARDING_STEPS = ['welcome', 'daemon', 'agent', 'model', 'vec0', 'done'] as const;
export const DEFAULT_ONBOARDING_STEPS = ['welcome', 'daemon', 'agent', 'model', 'done'] as const;
export type OnboardingStepId = (typeof ONBOARDING_STEPS)[number];

const ONBOARDING_KEY = 'onboarding';
const CURRENT_VERSION = 1;

interface OnboardingState {
  completed: boolean;
  completedAt?: string;
  version: number;
}

export interface UseOnboardingReturn {
  isFirstRun: Ref<boolean>;
  step: Ref<number>;
  highWaterStep: Ref<number>;
  currentStepId: ComputedRef<OnboardingStepId>;
  canGoBack: ComputedRef<boolean>;
  canGoForward: ComputedRef<boolean>;
  steps: ComputedRef<readonly OnboardingStepId[]>;
  checkFirstRun: () => boolean;
  advance: () => void;
  goToStep: (target: number) => void;
  goBack: () => void;
  markComplete: () => void;
}

function readOnboardingState(): OnboardingState | null {
  return readStorageValue<OnboardingState>(PREFS_STORE_PATH, ONBOARDING_KEY);
}

export function useOnboarding(
  stepSource: MaybeRefOrGetter<readonly OnboardingStepId[]> = DEFAULT_ONBOARDING_STEPS,
): UseOnboardingReturn {
  const isFirstRun = ref<boolean>(true);
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

  function checkFirstRun(): boolean {
    const state = readOnboardingState();
    if (!state || !state.completed || state.version < CURRENT_VERSION) {
      isFirstRun.value = true;
      return true;
    }
    isFirstRun.value = false;
    return false;
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
  }

  function goToStep(target: number): void {
    if (!Number.isInteger(target)) {
      return;
    }
    if (target < 0 || target > highWaterStep.value) {
      return;
    }
    step.value = target;
  }

  function goBack(): void {
    if (step.value <= 0) {
      return;
    }
    step.value -= 1;
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
    advance,
    goToStep,
    goBack,
    markComplete,
  };
}
