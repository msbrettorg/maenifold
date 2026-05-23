<script setup lang="ts">
import { computed, ref } from 'vue';
import { Brain, Search } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import HomeComposer from './HomeComposer.vue';
import HomeConnectionStatus from './HomeConnectionStatus.vue';
import HomeQuickTiles from './HomeQuickTiles.vue';
import HomeRecentActivity from './HomeRecentActivity.vue';
import { useDaemonHealth } from '../composables/useDaemonHealth';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import type { SavedSession } from '../lib/types';
import type { AppShellTab } from '../stores/app-shell';
import pkg from '../../package.json';

type ConnectionStatus = {
  label: string;
  description: string;
  toneClass: string;
  dotClass: string;
  actionLabel: string;
  action: string;
};

const appName = (pkg as { productName?: string; name: string }).productName ?? pkg.name;
const appVersion = pkg.version;

const props = defineProps<{
  hasAgents: boolean;
  selectedCwd: string;
  isConnected: boolean;
  isConnecting: boolean;
  recentSessions: SavedSession[];
  submitPrompt: (prompt: string) => Promise<boolean>;
}>();

const emit = defineEmits<{
  'agent-select': [agentName: string];
  'select-folder': [];
  'new-session': [];
  'select-tab': [tab: AppShellTab];
  'open-tab': [tab: AppShellTab];
  'resume-session': [session: SavedSession];
}>();

const daemonHealth = useDaemonHealth();
const { daemonOnline, lastErrorMessage } = daemonHealth;

const appShellStore = useAppShellStore();
const knowledgeStore = useKnowledgeStore();
const searchInput = ref('');
const canSearch = computed(() => searchInput.value.trim().length > 0);

function handleSearchSubmit(): void {
  const q = searchInput.value.trim();
  if (!q) return;
  knowledgeStore.setSearchQuery(q);
  appShellStore.setActiveTab('knowledge');
  searchInput.value = '';
}

const connectionStatus = computed<ConnectionStatus | null>(() => {
  if (!daemonOnline.value) {
    return {
      label: 'Daemon stopped',
      description: lastErrorMessage.value ?? 'Start maenifold --http, then retry.',
      toneClass: 'border-danger/30 bg-danger/10 text-danger',
      dotClass: 'bg-danger',
      actionLabel: 'Retry daemon',
      action: 'retry-daemon',
    };
  }
  if (props.isConnecting) {
    return {
      label: 'Connecting',
      description: 'Starting an agent session.',
      toneClass: 'border-warning/30 bg-warning/10 text-warning',
      dotClass: 'bg-warning',
      actionLabel: '',
      action: '',
    };
  }
  if (props.isConnected) {
    return {
      label: 'Connected',
      description: 'Ready to send work.',
      toneClass: 'border-success/30 bg-success/10 text-success',
      dotClass: 'bg-success',
      actionLabel: '',
      action: '',
    };
  }
  return null;
});

async function handleStatusAction(): Promise<void> {
  if (!connectionStatus.value) {
    return;
  }
  if (connectionStatus.value.action === 'retry-daemon') {
    await daemonHealth.forceCheck();
    return;
  }
  if (connectionStatus.value.action === 'start-session') {
    emit('new-session');
  }
}
</script>

<template>
  <div class="relative isolate flex flex-1 flex-col items-center overflow-hidden px-8 pt-[16vh]">
    <div class="relative z-10 flex w-full flex-col items-center">
      <div class="mb-5">
        <Brain class="size-16 text-text-tertiary opacity-40" :stroke-width="1" />
      </div>

      <div class="mb-3 flex items-center gap-2" aria-label="Application identity">
        <span class="text-sm font-semibold tracking-tight text-text-secondary">{{ appName }}</span>
        <span
          class="rounded-full border border-border-card bg-bg-elevated px-2 py-0.5 font-mono text-[10px] font-medium text-text-tertiary"
          aria-label="Application version"
        >v{{ appVersion }}</span>
      </div>

      <h1 class="mb-3 text-center text-[26px] font-semibold tracking-tight text-text-primary">What can I help you with?</h1>

      <HomeConnectionStatus
        :status="connectionStatus"
        @action="handleStatusAction"
      />

      <HomeQuickTiles
        @new-session="emit('new-session')"
        @select-tab="emit('select-tab', $event)"
        @open-tab="emit('open-tab', $event)"
      />

      <HomeComposer
        :daemon-online="daemonOnline"
        :is-connecting="isConnecting"
        :submit-prompt="submitPrompt"
        @select-tab="emit('select-tab', $event)"
        @open-tab="emit('open-tab', $event)"
      />

      <Card class="mt-2 w-full max-w-[800px] rounded-xl py-0 shadow-fluent-8 ring-0 border border-border-card bg-bg-elevated">
        <CardContent class="px-4 py-3">
          <form data-testid="home-search-form" class="flex items-center gap-2" @submit.prevent="handleSearchSubmit">
            <Search class="size-4 shrink-0 text-text-tertiary" :stroke-width="1.5" aria-hidden="true" />
            <Input
              v-model="searchInput"
              type="text"
              placeholder="Search notes..."
              aria-label="Search Knowledge"
              class="flex-1 border-0 bg-transparent text-sm text-text-primary shadow-none outline-none ring-0 placeholder:text-text-tertiary focus-visible:border-transparent focus-visible:ring-0 h-auto px-0 py-0 rounded-none"
            />
            <Button
              type="submit"
              variant="ghost"
              size="icon"
              :disabled="!canSearch"
              class="flex size-9 items-center justify-center rounded-[4px] text-text-tertiary transition-colors hover:text-text-primary disabled:opacity-40"
            >
              <Search class="size-4" :stroke-width="1.5" />
            </Button>
          </form>
        </CardContent>
      </Card>

      <!-- Keep returning-user resume paths close to the composer without displacing the launcher tiles. -->
      <HomeRecentActivity
        :recent-sessions="recentSessions"
        @open-tab="emit('open-tab', $event)"
        @resume-session="emit('resume-session', $event)"
      />
    </div>
  </div>
</template>
