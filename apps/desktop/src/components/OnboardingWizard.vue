<script setup lang="ts">
import { computed, onMounted, provide, ref, type Component } from 'vue';
import { Loader2 } from 'lucide-vue-next';

import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Stepper,
  StepperIndicator,
  StepperItem,
  StepperSeparator,
  StepperTitle,
  StepperTrigger,
} from '@/components/ui/stepper';

import {
  DEFAULT_ONBOARDING_STEPS,
  useOnboarding,
  type OnboardingStepId,
} from '../composables/useOnboarding';
import { useOnboardingVec0Download } from '../composables/useOnboardingVec0Download';
import OnboardingStepAgent from './OnboardingStepAgent.vue';
import OnboardingStepDaemon from './OnboardingStepDaemon.vue';
import OnboardingStepDone from './OnboardingStepDone.vue';
import OnboardingStepModel from './OnboardingStepModel.vue';
import OnboardingStepVec0 from './OnboardingStepVec0.vue';
import OnboardingStepWelcome from './OnboardingStepWelcome.vue';

const emit = defineEmits<{ complete: [] }>();

interface StepConfig {
  id: OnboardingStepId;
  label: string;
  component: Component;
}

const vec0 = useOnboardingVec0Download();

const stepIds = computed<readonly OnboardingStepId[]>(() =>
  vec0.needsDownload.value
    ? ['welcome', 'daemon', 'agent', 'model', 'vec0', 'done']
    : DEFAULT_ONBOARDING_STEPS,
);

const {
  step,
  highWaterStep,
  currentStepId,
  advance,
  goToStep,
  goBack,
  markComplete,
} = useOnboarding(stepIds);

const allSteps: Record<OnboardingStepId, StepConfig> = {
  welcome: { id: 'welcome', label: 'Welcome', component: OnboardingStepWelcome },
  daemon: { id: 'daemon', label: 'Daemon', component: OnboardingStepDaemon },
  agent: { id: 'agent', label: 'Agent', component: OnboardingStepAgent },
  model: { id: 'model', label: 'Model', component: OnboardingStepModel },
  vec0: { id: 'vec0', label: 'Vec0', component: OnboardingStepVec0 },
  done: { id: 'done', label: 'Finish', component: OnboardingStepDone },
};

const steps = computed<StepConfig[]>(() => stepIds.value.map((id) => allSteps[id]));
const wizardReady = computed<boolean>(() => !vec0.loading.value || !!vec0.loadError.value);

// Stepper expects 1-based modelValue; useOnboarding stores a 0-based step index.
const stepIndex = computed<number>(() => step.value + 1);

const activeStepComponent = computed<Component>(() => {
  return (
    steps.value.find((s) => s.id === currentStepId.value)?.component ??
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

function handleStepTrigger(target: number): void {
  if (target <= highWaterStep.value) {
    goToStep(target);
  }
}

provide('onboarding-advance', advance);
provide('onboarding-go-back', goBack);
provide('onboarding-mark-complete', handleMarkComplete);
provide('onboarding-vec0', vec0);

onMounted(() => {
  void vec0.loadStatus();
});
</script>

<template>
  <Dialog :open="true" :modal="true">
    <DialogContent
      :show-close-button="false"
      class="left-0 top-0 z-[1010] grid h-screen max-h-screen w-screen max-w-none translate-x-0 translate-y-0 gap-0 rounded-none border-0 bg-bg-app p-0 shadow-none"
      @interact-outside.prevent
      @escape-key-down.prevent
      @pointer-down-outside.prevent
    >
      <DialogTitle class="sr-only">Onboarding</DialogTitle>
      <ScrollArea class="h-full w-full">
        <div class="flex flex-col items-center justify-start min-h-screen p-8">
          <div class="mb-4 flex w-full max-w-2xl justify-end">
            <Button
              variant="ghost"
              size="sm"
              class="text-text-secondary"
              @click="handleMarkComplete"
            >
              Skip onboarding
            </Button>
          </div>

          <div
            v-if="!wizardReady"
            class="flex min-h-[50vh] w-full max-w-2xl flex-col items-center justify-center gap-3"
          >
            <Loader2 class="h-6 w-6 animate-spin text-brand" />
            <p class="text-sm text-text-secondary">Preparing onboarding…</p>
          </div>

          <Stepper
            v-else
            :model-value="stepIndex"
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
              <StepperTrigger @click="handleStepTrigger(i)">
                <StepperIndicator>{{ i + 1 }}</StepperIndicator>
                <StepperTitle class="text-xs">{{ item.label }}</StepperTitle>
              </StepperTrigger>
              <StepperSeparator
                v-if="i < steps.length - 1"
                class="h-0.5 flex-1"
              />
            </StepperItem>
          </Stepper>

          <div v-if="wizardReady" class="w-full max-w-2xl">
            <component :is="activeStepComponent" :key="currentStepId" />
          </div>
        </div>
      </ScrollArea>
    </DialogContent>
  </Dialog>
</template>
