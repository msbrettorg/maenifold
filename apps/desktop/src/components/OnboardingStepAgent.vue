<script setup lang="ts">
import { computed, inject, onUnmounted, ref } from 'vue';
import { Bot } from 'lucide-vue-next';

import { useConfigStore } from '../stores/config';

interface RuntimeOption {
  id: string;
  label: string;
  tag: string | null;
  desc: string;
  command: string;
}

const RUNTIMES: RuntimeOption[] = [
  {
    id: 'copilot-cli',
    label: 'GitHub Copilot CLI',
    tag: 'popular',
    desc: 'Free with GitHub Copilot subscription',
    command: 'copilot-cli',
  },
  {
    id: 'claude',
    label: 'Claude (Anthropic)',
    tag: null,
    desc: 'Requires API key',
    command: 'claude',
  },
  {
    id: 'openai',
    label: 'OpenAI GPT',
    tag: null,
    desc: 'Requires API key',
    command: 'openai',
  },
];

const advance = inject<() => void>('onboarding-advance');
const goBack = inject<() => void>('onboarding-go-back');

const configStore = useConfigStore();

const agentName = ref('Copilot');
const selectedRuntime = ref<RuntimeOption | null>(RUNTIMES[0] ?? null);
const errorMessage = ref<string | null>(null);
const submitting = ref(false);

let disposed = false;

const trimmedName = computed(() => agentName.value.trim());

const duplicateName = computed(() => {
  if (trimmedName.value.length === 0) return false;
  return configStore.agentNames.includes(trimmedName.value);
});

const isValid = computed(() => {
  return trimmedName.value.length > 0 && selectedRuntime.value !== null && !duplicateName.value;
});

function selectRuntime(runtime: RuntimeOption): void {
  selectedRuntime.value = runtime;
}

async function onSubmit(): Promise<void> {
  errorMessage.value = null;

  if (trimmedName.value.length === 0) {
    errorMessage.value = 'Agent name is required.';
    return;
  }
  if (duplicateName.value) {
    errorMessage.value = `An agent named "${trimmedName.value}" already exists.`;
    return;
  }
  if (selectedRuntime.value === null) {
    errorMessage.value = 'Select a runtime.';
    return;
  }

  submitting.value = true;
  try {
    await configStore.addAgent(trimmedName.value, {
      command: selectedRuntime.value.command,
    });
    if (disposed) return;
    advance?.();
  } catch (e) {
    if (disposed) return;
    errorMessage.value = e instanceof Error ? e.message : String(e);
  } finally {
    if (!disposed) {
      submitting.value = false;
    }
  }
}

function onBack(): void {
  goBack?.();
}

onUnmounted(() => {
  disposed = true;
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-brand/10 text-brand">
      <Bot class="h-6 w-6" />
    </div>

    <div class="flex flex-col gap-2">
      <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
        Add your first agent
      </h1>
      <p class="text-sm text-text-secondary leading-relaxed">
        Choose an agent runtime and give it a name. You can add more agents later.
      </p>
    </div>

    <div class="flex flex-col gap-2">
      <label for="agent-name" class="text-xs font-medium text-text-secondary">
        Agent name
      </label>
      <input
        id="agent-name"
        v-model="agentName"
        type="text"
        placeholder="Agent name"
        class="bg-bg-surface border border-border-subtle rounded-lg px-3.5 py-2.5 text-sm text-text-primary font-mono placeholder:text-text-tertiary focus:border-brand focus:outline-none focus:ring-1 focus:ring-brand/50"
      />
    </div>

    <div class="flex flex-col gap-2">
      <span class="text-xs font-medium text-text-secondary">Runtime</span>
      <div class="flex flex-col gap-2">
        <button
          v-for="runtime in RUNTIMES"
          :key="runtime.id"
          type="button"
          class="bg-bg-surface border border-border-subtle rounded-xl p-4 cursor-pointer text-left transition-colors hover:bg-bg-surface-hover hover:border-text-tertiary"
          :class="{
            'border-brand bg-brand/5': selectedRuntime?.id === runtime.id,
          }"
          @click="selectRuntime(runtime)"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="flex flex-col gap-1">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium text-text-primary">{{ runtime.label }}</span>
                <span
                  v-if="runtime.tag"
                  class="text-2xs bg-brand/10 text-brand rounded px-1.5 py-0.5"
                >
                  {{ runtime.tag }}
                </span>
              </div>
              <span class="text-xs text-text-tertiary">{{ runtime.desc }}</span>
            </div>
            <span
              class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full border"
              :class="
                selectedRuntime?.id === runtime.id
                  ? 'border-brand bg-brand'
                  : 'border-border-subtle bg-bg-surface'
              "
            >
              <span
                v-if="selectedRuntime?.id === runtime.id"
                class="h-1.5 w-1.5 rounded-full bg-primary-foreground"
              />
            </span>
          </div>
        </button>
      </div>
    </div>

    <p v-if="errorMessage" class="text-danger text-sm">{{ errorMessage }}</p>

    <div class="flex items-center justify-between gap-3 pt-2">
      <button
        type="button"
        class="rounded-lg border border-border-subtle bg-bg-surface px-4 py-2 text-sm font-medium text-text-secondary transition-colors hover:bg-bg-elevated"
        @click="onBack"
      >
        ← Back
      </button>

      <button
        type="button"
        :disabled="!isValid || submitting"
        class="rounded-lg border-none bg-brand px-6 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
        @click="onSubmit"
      >
        {{ submitting ? 'Adding…' : 'Add agent →' }}
      </button>
    </div>
  </div>
</template>
