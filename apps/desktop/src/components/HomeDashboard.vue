<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { MessageSquare, GitBranch, Shield, Brain, Search, Send, Pencil } from 'lucide-vue-next';
import type { Component } from 'vue';
import { usePipelineStore } from '../stores/pipeline';
import { useGateMcp } from '../composables/useGateMcp';
import { useGateRegistry } from '../composables/useGateRegistry';
import type { GateSnapshot } from '../lib/gate';
import type { SavedSession } from '../lib/types';
import type { AppShellTab } from '../stores/app-shell';

interface HomeTile {
  key: string;
  title: string;
  description: string;
  icon: Component;
  accentClass: string;
  tab?: AppShellTab;
  action?: string;
}

const props = defineProps<{
  hasAgents: boolean;
  selectedCwd: string;
  isConnected: boolean;
  isConnecting: boolean;
  recentSessions: SavedSession[];
}>();

const emit = defineEmits<{
  'agent-select': [agentName: string];
  'select-folder': [];
  'new-session': [];
  'prompt-submit': [prompt: string];
  'select-tab': [tab: AppShellTab];
  'open-tab': [tab: AppShellTab];
  'resume-session': [session: SavedSession];
}>();

const pipelineStore = usePipelineStore();
const gate = useGateMcp();
const gateRegistry = useGateRegistry(() => props.selectedCwd);
const gateSnapshot = ref<GateSnapshot | null>(null);
const composerInput = ref('');

const categories: HomeTile[] = [
  { key: 'agents', title: 'Agents', description: 'Start or resume sessions', icon: MessageSquare, accentClass: 'text-brand', tab: 'agents' },
  { key: 'pipelines', title: 'Pipelines', description: 'Run multi-step workflows', icon: GitBranch, accentClass: 'text-success', tab: 'pipelines' },
  { key: 'gate', title: 'Gate', description: 'Track tasks & delivery', icon: Shield, accentClass: 'text-warning', tab: 'gate' },
  { key: 'workflows', title: 'Workflows', description: 'Browse & author', icon: Brain, accentClass: 'text-thinking', tab: 'workflows' },
  { key: 'knowledge', title: 'Knowledge', description: 'Explore the graph', icon: Search, accentClass: 'text-brand', tab: 'knowledge' },
];

const daemonConnected = computed(() => props.isConnected);
const pendingGateTasks = computed(
  () => (gateSnapshot.value?.tasks ?? []).filter((task) => task.location === 'todo' && task.state === 'pending').length,
);

async function refreshGateSnapshot() {
  const root = gateRegistry.selectedGateRoot.value;
  if (!root) {
    gateSnapshot.value = null;
    return;
  }
  try {
    gateSnapshot.value = await gate.loadSnapshot(root);
  } catch {
    gateSnapshot.value = null;
  }
}

function handleTileSelect(tile: HomeTile) {
  if (tile.action === 'new-session') {
    emit('new-session');
    return;
  }
  if (tile.tab) {
    emit('select-tab', tile.tab);
    emit('open-tab', tile.tab);
  }
}

function handleComposerSubmit() {
  const trimmedPrompt = composerInput.value.trim();
  if (!trimmedPrompt) return;
  emit('prompt-submit', trimmedPrompt);
  emit('select-tab', 'agents');
  emit('open-tab', 'agents');
  composerInput.value = '';
}

watch(
  () => gateRegistry.selectedGateRoot.value,
  async (root) => {
    if (!root) {
      gate.stopPolling();
      gateSnapshot.value = null;
      return;
    }
    await refreshGateSnapshot();
    gate.startPolling(root, refreshGateSnapshot);
  },
  { immediate: true },
);

onMounted(async () => {
  await Promise.allSettled([pipelineStore.refreshList(), refreshGateSnapshot()]);
});

onBeforeUnmount(() => {
  gate.stopPolling();
  void gate.disconnect();
});
</script>

<template>
  <div class="flex flex-1 flex-col items-center px-8 pt-[16vh]">
    <div class="mb-5">
      <Brain class="size-16 text-text-tertiary opacity-40" :stroke-width="1" />
    </div>

    <h1 class="mb-3 text-center text-[26px] font-semibold tracking-tight text-text-primary">What can I help you with?</h1>

    <div class="mb-4 grid w-full max-w-[800px] grid-cols-5 gap-2">
      <button
        v-for="cat in categories"
        :key="cat.key"
        type="button"
        class="flex min-h-[72px] flex-col items-center justify-center gap-1.5 rounded-[8px] border-[1.5px] border-border-card bg-bg-elevated px-1 py-3.5 transition-all hover:bg-bg-surface-hover"
        @click="handleTileSelect(cat)"
      >
        <component :is="cat.icon" class="size-5" :class="cat.accentClass" :stroke-width="1.5" />
        <span class="text-center text-[11px] font-semibold text-text-secondary">{{ cat.title }}</span>
        <span class="text-center text-[9px] leading-tight text-text-tertiary">{{ cat.description }}</span>
      </button>
    </div>

    <div class="mb-4 flex flex-wrap justify-center gap-1.5">
      <div class="flex items-center gap-1.5 rounded-[var(--radius-4)] border border-border-subtle px-2.5 py-1 text-xs text-text-secondary">
        <span class="size-1.5 rounded-full" :class="daemonConnected ? 'bg-success' : 'bg-danger'" />
        {{ daemonConnected ? 'Connected' : 'Disconnected' }}
      </div>
      <div
        v-if="pendingGateTasks > 0"
        class="flex items-center gap-1.5 rounded-[var(--radius-4)] border border-border-subtle px-2.5 py-1 text-xs text-text-secondary"
      >
        <Shield class="size-3" :stroke-width="1.5" />
        {{ pendingGateTasks }} pending
      </div>
    </div>

    <div class="w-full max-w-[800px] rounded-xl border border-border-card bg-bg-elevated px-4 py-3 shadow-[var(--shadow-8)]">
      <form class="flex items-center gap-2" @submit.prevent="handleComposerSubmit">
        <Pencil class="size-4 shrink-0 text-text-tertiary" :stroke-width="1.5" />
        <input
          v-model="composerInput"
          type="text"
          placeholder="Ask me anything..."
          class="flex-1 bg-transparent text-sm text-text-primary outline-none placeholder:text-text-tertiary"
        />
        <button
          type="submit"
          class="flex size-9 items-center justify-center rounded-[var(--radius-4)] text-text-tertiary transition-colors hover:text-text-primary"
        >
          <Send class="size-4" :stroke-width="1.5" />
        </button>
      </form>
    </div>
  </div>
</template>
