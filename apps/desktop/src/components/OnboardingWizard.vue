<script setup lang="ts">
import { computed, provide, ref, type Component } from 'vue';

import {
  Stepper,
  StepperIndicator,
  StepperItem,
  StepperSeparator,
  StepperTitle,
  StepperTrigger,
} from '@/components/ui/stepper';

import { useOnboarding, type OnboardingStepId } from '../composables/useOnboarding';
import OnboardingStepAgent from './OnboardingStepAgent.vue';
import OnboardingStepDaemon from './OnboardingStepDaemon.vue';
import OnboardingStepDone from './OnboardingStepDone.vue';
import OnboardingStepModel from './OnboardingStepModel.vue';
import OnboardingStepWelcome from './OnboardingStepWelcome.vue';

const emit = defineEmits<{ complete: [] }>();

const {
  step,
  highWaterStep,
  currentStepId,
  advance,
  goToStep,
  goBack,
  markComplete,
} = useOnboarding();

interface StepConfig {
  id: OnboardingStepId;
  label: string;
  component: Component;
}

const steps: StepConfig[] = [
  { id: 'welcome', label: 'Welcome', component: OnboardingStepWelcome },
  { id: 'daemon', label: 'Daemon', component: OnboardingStepDaemon },
  { id: 'agent', label: 'Agent', component: OnboardingStepAgent },
  { id: 'model', label: 'Model', component: OnboardingStepModel },
  { id: 'done', label: 'Finish', component: OnboardingStepDone },
];

// Bridge useOnboarding (0-based step) <-> Stepper v-model (1-based).
const stepIndex = computed<number>({
  get: () => step.value + 1,
  set: (value) => {
    const target = value - 1;
    if (target >= 0 && target <= highWaterStep.value) {
      goToStep(target);
    }
  },
});

const activeStepComponent = computed<Component>(() => {
  return (
    steps.find((s) => s.id === currentStepId.value)?.component ??
    OnboardingStepWelcome
  );
});

const modelSkipped = ref(false);
const selectedModelName = ref<string | null>(null);
provide('onboarding-model-skipped', modelSkipped);
provide('onboarding-selected-model', selectedModelName);

function handleMarkComplete(): void {
  markComplete();
  emit('complete');
}

provide('onboarding-advance', advance);
provide('onboarding-go-back', goBack);
provide('onboarding-mark-complete', handleMarkComplete);
</script>

<template>
  <div class="fixed inset-0 z-50 flex flex-col items-center justify-start min-h-screen p-8 bg-bg-app overflow-y-auto">
    <Stepper
      v-model="stepIndex"
      :linear="false"
      class="w-full max-w-2xl mb-8 flex items-start justify-between gap-2"
    >
      <StepperItem
        v-for="(item, i) in steps"
        :key="item.id"
        :step="i + 1"
        :disabled="i > highWaterStep"
        class="flex items-center gap-2 flex-1 last:flex-none"
      >
        <StepperTrigger>
          <StepperIndicator>{{ i + 1 }}</StepperIndicator>
          <StepperTitle class="text-xs">{{ item.label }}</StepperTitle>
        </StepperTrigger>
        <StepperSeparator
          v-if="i < steps.length - 1"
          class="h-0.5 flex-1"
        />
      </StepperItem>
    </Stepper>

    <div class="w-full max-w-2xl">
      <component :is="activeStepComponent" :key="currentStepId" />
    </div>
  </div>
</template>
