<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref } from 'vue';
import { ClipboardCopy, Plug } from 'lucide-vue-next';

import { DAEMON_BASE_URL } from '../lib/daemon-url';
import { useConfigStore } from '../stores/config';

type ProbeState = 'checking' | 'connected' | 'disconnected';

const advance = inject<() => void>('onboarding-advance');
const goBack = inject<() => void>('onboarding-go-back');

const configStore = useConfigStore();

const state = ref<ProbeState>('checking');
const copied = ref(false);
const command = 'maenifold --http';

const agentCount = computed(() => configStore.agentNames.length);
const mcpServerCount = computed(() => (configStore.config.mcpServers ?? []).length);

let pollTimer: ReturnType<typeof setInterval> | null = null;
let copyTimer: ReturnType<typeof setTimeout> | null = null;
let disposed = false;

async function checkDaemonHealth(): Promise<boolean> {
  try {
    await configStore.loadConfig();
    return configStore.error === null;
  } catch {
    return false;
  }
}

async function runProbe(): Promise<void> {
  state.value = 'checking';
  const ok = await checkDaemonHealth();
  if (disposed) return;
  if (ok) {
    state.value = 'connected';
    stopPolling();
  } else {
    state.value = 'disconnected';
    startPolling();
  }
}

function startPolling(): void {
  if (pollTimer !== null) return;
  pollTimer = setInterval(async () => {
    const ok = await checkDaemonHealth();
    if (disposed) return;
    if (ok) {
      state.value = 'connected';
      stopPolling();
    }
  }, 5000);
}

function stopPolling(): void {
  if (pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function onCopy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(command);
    copied.value = true;
    if (copyTimer !== null) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    // Clipboard not available; ignore.
  }
}

function onCheckConnection(): void {
  void runProbe();
}

function onContinue(): void {
  if (state.value === 'connected') {
    advance?.();
  }
}

function onBack(): void {
  goBack?.();
}

onMounted(() => {
  void runProbe();
});

onUnmounted(() => {
  disposed = true;
  stopPolling();
  if (copyTimer !== null) clearTimeout(copyTimer);
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-brand/10 text-brand">
      <Plug class="h-6 w-6" />
    </div>

    <div class="flex flex-col gap-2">
      <h1 class="text-[28px] font-bold tracking-tight text-text-primary">
        Connect to the daemon
      </h1>
      <p class="text-sm text-text-secondary leading-relaxed">
        maenifold needs the local daemon running to serve tools, agents, and memory.
      </p>
    </div>

    <!-- Checking -->
    <div
      v-if="state === 'checking'"
      class="flex items-center gap-3 rounded-xl border border-border-subtle bg-bg-surface p-4"
    >
      <span class="relative flex h-2.5 w-2.5">
        <span
          class="absolute inline-flex h-full w-full animate-ping rounded-full bg-brand opacity-75"
        />
        <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-brand" />
      </span>
      <span class="text-sm text-text-secondary">Checking connection...</span>
    </div>

    <!-- Disconnected -->
    <div v-else-if="state === 'disconnected'" class="flex flex-col gap-3">
      <div class="flex items-center gap-3 rounded-xl border border-border-subtle bg-bg-surface p-4">
        <span class="inline-flex h-2.5 w-2.5 rounded-full bg-danger" />
        <div class="flex flex-col">
          <span class="text-sm font-medium text-danger">Disconnected</span>
          <span class="text-xs text-text-tertiary">Expected at: {{ DAEMON_BASE_URL }}</span>
        </div>
      </div>

      <p class="text-sm text-text-secondary">
        Start the daemon by running this command in your terminal:
      </p>

      <div
        class="flex items-center justify-between gap-3 rounded-lg border border-border-subtle bg-bg-surface p-3 font-mono text-sm text-text-secondary"
      >
        <code class="truncate">{{ command }}</code>
        <button
          type="button"
          class="flex shrink-0 items-center gap-1.5 rounded-md border border-border-subtle bg-bg-elevated px-2 py-1 font-sans text-xs text-text-secondary transition-colors hover:bg-bg-surface"
          @click="onCopy"
        >
          <template v-if="copied">
            <span>✓ Copied!</span>
          </template>
          <template v-else>
            <ClipboardCopy class="h-3.5 w-3.5" />
            <span>Copy</span>
          </template>
        </button>
      </div>
    </div>

    <!-- Connected -->
    <div
      v-else
      class="flex flex-col gap-2 rounded-xl border border-success bg-success/10 p-4"
    >
      <span class="text-sm font-medium text-text-primary">✓ Connected to daemon</span>
      <div class="flex flex-col gap-0.5 text-xs text-text-secondary">
        <span>Agents: {{ agentCount }}</span>
        <span>MCP servers: {{ mcpServerCount }}</span>
      </div>
    </div>

    <div class="flex items-center justify-between gap-3 pt-2">
      <button
        type="button"
        class="rounded-lg border border-border-subtle bg-bg-surface px-4 py-2 text-sm font-medium text-text-secondary transition-colors hover:bg-bg-elevated"
        @click="onBack"
      >
        ← Back
      </button>

      <div class="flex items-center gap-2">
        <button
          v-if="state !== 'connected'"
          type="button"
          :disabled="state === 'checking'"
          class="rounded-lg border border-border-subtle bg-bg-surface px-4 py-2 text-sm font-medium text-text-secondary transition-colors hover:bg-bg-elevated disabled:cursor-not-allowed disabled:opacity-50"
          @click="onCheckConnection"
        >
          Check connection
        </button>

        <button
          type="button"
          :disabled="state !== 'connected'"
          class="rounded-lg border-none bg-brand px-6 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
          @click="onContinue"
        >
          Continue →
        </button>
      </div>
    </div>
  </div>
</template>
