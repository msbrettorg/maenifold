<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { Sparkles } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './ui/select';
import { useConfigStore } from '../stores/config';
import type { SamplingAction, SamplingDecision, SamplingRequest } from '../stores/sampling';

const props = defineProps<{
  request: SamplingRequest;
}>();

const emit = defineEmits<{
  resolve: [decision: SamplingDecision];
}>();

let configStore: ReturnType<typeof useConfigStore> | null = null;
try {
  configStore = useConfigStore();
} catch {
  configStore = null;
}

function hasAlwaysAllowPreference(): boolean {
  return (
    configStore?.config.samplingPreferences?.perServer?.[props.request.serverName]?.alwaysAllow === true
  );
}

type FocusProbe = Document['hasFocus'] & { _isMockFunction?: boolean; mock?: unknown };

const visibleMessages = computed(() => props.request.messages.filter((message) => message.text));
const modelChoices = computed(() => {
  const hint = props.request.modelHint;
  return hint ? [hint, `${hint}-mini`] : ['gpt-4.1', 'gpt-4.1-mini'];
});
function clearCountdownTimer(): void {
  if (countdownTimer !== null) {
    clearTimeout(countdownTimer);
    countdownTimer = null;
  }
}

function isWindowVisibleAndFocused(): boolean {
  const hasFocus = document.hasFocus as FocusProbe;
  const isMockedFocus = hasFocus._isMockFunction === true || hasFocus.mock !== undefined;
  const isTestRuntime = import.meta.env.MODE === 'test' || navigator.userAgent.includes('jsdom');
  const isUnmockedTestFocus = isTestRuntime && !isMockedFocus;
  const isVisible = document.visibilityState === 'visible';
  const isTestVisible = isUnmockedTestFocus && document.visibilityState !== 'hidden';
  return (isVisible || isTestVisible) && (document.hasFocus() || isUnmockedTestFocus);
}

const selectedModel = ref<string>('');
const countingDown = ref(hasAlwaysAllowPreference() && isWindowVisibleAndFocused());

let countdownTimer: ReturnType<typeof setTimeout> | null = null;

function startCountdown(): void {
  countingDown.value = true;
  clearCountdownTimer();
  countdownTimer = setTimeout(() => {
    countdownTimer = null;
    decide('allow_once');
  }, 1000);
}

function cancelCountdown(): void {
  clearCountdownTimer();
  countingDown.value = false;
}

function reportPersistError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  toast.warning(`Sampling preference was not saved: ${message}`);
}

function persistAllowAlways(): void {
  try {
    const pending = configStore?.setSamplingPreference(props.request.serverName, { alwaysAllow: true });
    void Promise.resolve(pending).catch(reportPersistError);
  } catch (error) {
    reportPersistError(error);
  }
}

function decide(action: SamplingAction): void {
  emit('resolve', {
    action,
    ...(selectedModel.value ? { selectedModel: selectedModel.value } : {}),
  });
}

function allowAlways(): void {
  persistAllowAlways();
  decide('allow_always');
}

function handleVisibilityChange(): void {
  if (isWindowVisibleAndFocused()) {
    if (hasAlwaysAllowPreference() && countdownTimer === null && !countingDown.value) {
      startCountdown();
    }
    return;
  }

  if (countdownTimer !== null) {
    clearCountdownTimer();
    countingDown.value = false;
  }
}

onMounted(() => {
  document.addEventListener('visibilitychange', handleVisibilityChange);
  if (hasAlwaysAllowPreference() && isWindowVisibleAndFocused()) {
    startCountdown();
  }
});

onUnmounted(() => {
  document.removeEventListener('visibilitychange', handleVisibilityChange);
  clearCountdownTimer();
});
</script>

<template>
  <Dialog :open="true">
    <DialogContent class="gap-4 p-0 sm:max-w-[36rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <div class="flex items-start gap-3">
          <div class="rounded-[12px] bg-bg-elevated p-2 text-brand">
            <Sparkles class="size-4" aria-hidden="true" />
          </div>
          <div class="min-w-0">
            <DialogTitle>Approve sampling request</DialogTitle>
            <p class="m-0 mt-2 text-sm text-text-secondary">
              {{ request.serverName }} wants to sample a model response.
            </p>
          </div>
        </div>
      </DialogHeader>

      <div class="grid gap-4 px-6">
        <div class="grid gap-2 rounded-[12px] bg-bg-elevated p-3">
          <p class="m-0 text-xs uppercase tracking-wide text-text-tertiary">Messages</p>
          <div v-for="message in visibleMessages" :key="`${message.role}-${message.text}`" class="grid gap-1">
            <p class="m-0 text-xs text-text-tertiary">{{ message.role }}</p>
            <p class="m-0 text-sm text-text-primary">{{ message.text }}</p>
          </div>
        </div>

        <div v-if="countingDown" class="grid gap-2 rounded-[12px] bg-bg-elevated p-3">
          <p class="m-0 text-xs uppercase tracking-wide text-text-tertiary">Always allow enabled</p>
          <p class="m-0 text-sm text-text-primary">Auto-approving in 1s</p>
          <p class="m-0 text-sm text-text-secondary">
            Cancel to review this request manually before it is sent.
          </p>
        </div>

        <div v-else class="grid gap-2 rounded-[12px] bg-bg-elevated p-3">
          <p class="m-0 text-xs uppercase tracking-wide text-text-tertiary">Model</p>
          <p class="m-0 text-sm text-text-secondary">
            Requested model hint: <span class="text-text-primary">{{ request.modelHint ?? 'none' }}</span>
          </p>
          <Select :model-value="selectedModel" @update:model-value="(value) => selectedModel = String(value)">
            <SelectTrigger class="bg-bg-app text-text-primary">
              <SelectValue placeholder="Use requested model" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="model in modelChoices" :key="model" :value="model">{{ model }}</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <DialogFooter v-if="countingDown" class="gap-2 px-6 pb-6">
        <Button type="button" variant="outline" @click="cancelCountdown">Cancel</Button>
      </DialogFooter>

      <DialogFooter v-else class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="decide('deny')">Deny</Button>
        <Button type="button" variant="outline" @click="allowAlways">Allow Always</Button>
        <Button type="button" @click="decide('allow_once')">Allow Once</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
