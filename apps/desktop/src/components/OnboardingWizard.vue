<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { computed, provide, ref } from 'vue';
import { Check } from 'lucide-vue-next';

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
  ONBOARDING_STEPS,
} = useOnboarding();

const stepLabels: Record<OnboardingStepId, string> = {
  welcome: 'Welcome',
  daemon: 'Daemon',
  agent: 'Agent',
  model: 'Model',
  done: 'Finish',
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const stepComponents: Record<OnboardingStepId, any | null> = {
  welcome: OnboardingStepWelcome,
  daemon: OnboardingStepDaemon,
  agent: OnboardingStepAgent,
  model: OnboardingStepModel,
  done: OnboardingStepDone,
};

const modelSkipped = ref(false);
const selectedModelName = ref<string | null>(null);
provide('onboarding-model-skipped', modelSkipped);
provide('onboarding-selected-model', selectedModelName);

const activeStepComponent = computed(() => {
  return stepComponents[currentStepId.value] ?? OnboardingStepWelcome;
});

function handleMarkComplete(): void {
  markComplete();
  emit('complete');
}

provide('onboarding-advance', advance);
provide('onboarding-go-back', goBack);
provide('onboarding-mark-complete', handleMarkComplete);

function stepState(index: number): 'complete' | 'current' | 'pending' {
  if (index < step.value) return 'complete';
  if (index === step.value) return 'current';
  return 'pending';
}

function onStepClick(index: number): void {
  if (index <= highWaterStep.value) {
    goToStep(index);
  }
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex bg-bg-app">
    <!-- Left sidebar -->
    <aside class="flex w-60 shrink-0 flex-col bg-bg-sidebar pt-[52px] px-5 pb-8">
      <div class="mb-8 text-sm font-semibold text-text-primary">maenifold</div>

      <nav class="flex flex-col gap-1">
        <Button
          v-for="(stepId, index) in ONBOARDING_STEPS"
          :key="stepId"
          type="button"
          :disabled="index > highWaterStep"
          class="flex items-center gap-3 rounded-md px-2 py-1.5 text-left transition-colors"
          :class="[
            index <= highWaterStep ? 'cursor-pointer hover:bg-bg-surface/60' : 'cursor-default',
          ]"
          @click="onStepClick(index)"
        >
          <span
            class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold transition-all"
            :class="{
              'bg-success text-primary-foreground': stepState(index) === 'complete',
              'bg-brand text-primary-foreground ring-2 ring-brand/30': stepState(index) === 'current',
              'bg-bg-surface text-text-tertiary': stepState(index) === 'pending',
            }"
          >
            <Check v-if="stepState(index) === 'complete'" class="h-3 w-3" />
            <span v-else>{{ index + 1 }}</span>
          </span>
          <span
            class="text-xs"
            :class="{
              'text-text-primary font-medium': stepState(index) === 'current',
              'text-text-secondary': stepState(index) === 'complete',
              'text-text-tertiary': stepState(index) === 'pending',
            }"
          >
            {{ stepLabels[stepId] }}
          </span>
        </Button>
      </nav>
    </aside>

    <!-- Right main area -->
    <main class="flex flex-1 overflow-y-auto bg-bg-app p-10">
      <div class="mx-auto w-full max-w-[640px]">
        <Transition name="wizard-step" mode="out-in">
          <component :is="activeStepComponent" :key="currentStepId" />
        </Transition>
      </div>
    </main>
  </div>
</template>

<style scoped>
.wizard-step-enter-active,
.wizard-step-leave-active {
  transition: opacity 0.35s ease, transform 0.35s ease;
}

.wizard-step-enter-from {
  opacity: 0;
  transform: translateY(12px);
}

.wizard-step-leave-to {
  opacity: 0;
  transform: translateY(-12px);
}
</style>
