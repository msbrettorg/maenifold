<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { computed, inject, onUnmounted, ref } from 'vue';
import { Bot } from 'lucide-vue-next';

import { useConfigStore } from '../stores/config';

const advance = inject<() => void>('onboarding-advance');
const goBack = inject<() => void>('onboarding-go-back');

const configStore = useConfigStore();

const alreadyConfigured = ref(configStore.hasAgents);
const existingAgentNames = ref<string[]>([...configStore.agentNames]);

const agentName = ref('Copilot');
const agentCommand = ref('');
const errorMessage = ref<string | null>(null);
const submitting = ref(false);

let disposed = false;

const trimmedName = computed(() => agentName.value.trim());
const trimmedCommand = computed(() => agentCommand.value.trim());

const duplicateName = computed(() => {
  if (trimmedName.value.length === 0) return false;
  return configStore.agentNames.includes(trimmedName.value);
});

const isValid = computed(() => {
  return (
    trimmedName.value.length > 0 &&
    trimmedCommand.value.length > 0 &&
    !duplicateName.value
  );
});

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
  if (trimmedCommand.value.length === 0) {
    errorMessage.value = 'Agent command is required.';
    return;
  }

  submitting.value = true;
  try {
    await configStore.addAgent(trimmedName.value, {
      command: trimmedCommand.value,
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

function onContinue(): void {
  advance?.();
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

    <template v-if="alreadyConfigured">
      <div class="flex flex-col gap-2">
        <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
          Agent already configured
        </h1>
        <p class="text-sm text-text-secondary leading-relaxed">
          You already have {{ existingAgentNames.length }}
          {{ existingAgentNames.length === 1 ? 'agent' : 'agents' }} configured. You can
          continue, or add more agents later from settings.
        </p>
      </div>

      <div class="flex flex-col gap-2">
        <span class="text-xs font-medium text-text-secondary">Configured agents</span>
        <ul class="flex flex-col gap-1.5">
          <li
            v-for="name in existingAgentNames"
            :key="name"
            class="bg-bg-surface border border-border-subtle rounded-lg px-3.5 py-2.5 text-sm text-text-primary font-mono"
          >
            {{ name }}
          </li>
        </ul>
      </div>

      <div class="flex items-center justify-between gap-3 pt-2">
        <Button
          type="button"
          class="rounded-lg border border-border-subtle bg-bg-surface px-4 py-2 text-sm font-medium text-text-secondary transition-colors hover:bg-bg-elevated"
          @click="onBack"
        >
          ← Back
        </Button>

        <Button
          type="button"
          class="rounded-lg border-none bg-brand px-6 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-brand-hover"
          @click="onContinue"
        >
          Continue →
        </Button>
      </div>
    </template>

    <template v-else>
      <div class="flex flex-col gap-2">
        <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
          Add your first agent
        </h1>
        <p class="text-sm text-text-secondary leading-relaxed">
          Give your agent a name and the shell command used to launch it. You can add
          more agents later.
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
        <label for="agent-command" class="text-xs font-medium text-text-secondary">
          Command
        </label>
        <input
          id="agent-command"
          v-model="agentCommand"
          type="text"
          placeholder="npx @github/copilot-language-server@latest --acp"
          class="bg-bg-surface border border-border-subtle rounded-lg px-3.5 py-2.5 text-sm text-text-primary font-mono placeholder:text-text-tertiary focus:border-brand focus:outline-none focus:ring-1 focus:ring-brand/50"
        />
        <small class="text-xs text-text-tertiary">
          Full shell command including arguments.
        </small>
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
    </template>
  </div>
</template>
