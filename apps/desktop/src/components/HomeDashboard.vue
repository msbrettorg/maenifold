<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Brain, Search, Send, Pencil } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import HomeRecentActivity from './HomeRecentActivity.vue';
import { useDaemonHealth } from '../composables/useDaemonHealth';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import type { SavedSession } from '../lib/types';
import type { AppShellTab } from '../stores/app-shell';
import pkg from '../../package.json';

const appName = (pkg as { productName?: string; name: string }).productName ?? pkg.name;
const appVersion = pkg.version;

interface HomeTile {
  key: string;
  title: string;
  description: string;
  icon: string;
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

const composerInput = ref('');
const composerError = ref('');
const isSubmittingPrompt = ref(false);
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

const connectionStatus = computed(() => {
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
  return {
    label: 'Agent stopped',
    description: props.hasAgents ? 'Start a session before sending work.' : 'No agents are available yet.',
    toneClass: 'border-border-card bg-bg-surface text-text-secondary',
    dotClass: 'bg-text-muted',
    actionLabel: props.hasAgents ? 'Start session' : '',
    action: props.hasAgents ? 'start-session' : '',
  };
});

const canSubmitPrompt = computed(() => (
  !isSubmittingPrompt.value
  && daemonOnline.value
  && !props.isConnecting
));

const categories: HomeTile[] = [
  { key: 'agents', title: 'Agents', description: 'Start or resume sessions', icon: '💬', accentClass: 'text-brand', tab: 'agents' },
  { key: 'pipelines', title: 'Pipelines', description: 'Run multi-step workflows', icon: '🔀', accentClass: 'text-success', tab: 'pipelines' },
  { key: 'gate', title: 'Gate', description: 'Track tasks & delivery', icon: '📋', accentClass: 'text-warning', tab: 'gate' },
  { key: 'workflows', title: 'Workflows', description: 'Browse & author', icon: '⚡', accentClass: 'text-thinking', tab: 'workflows' },
  { key: 'knowledge', title: 'Knowledge', description: 'Explore the graph', icon: '🧠', accentClass: 'text-brand', tab: 'knowledge' },
];

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

async function handleComposerSubmit() {
  const trimmedPrompt = composerInput.value.trim();
  if (!trimmedPrompt) return;
  if (!canSubmitPrompt.value) return;

  composerError.value = '';
  isSubmittingPrompt.value = true;

  try {
    const sent = await props.submitPrompt(trimmedPrompt);
    if (!sent) {
      composerError.value = 'Unable to send your prompt. Check the session connection and try again.';
      return;
    }
  } catch {
    composerError.value = 'Unable to send your prompt. Check the session connection and try again.';
    return;
  } finally {
    isSubmittingPrompt.value = false;
  }

  emit('select-tab', 'agents');
  emit('open-tab', 'agents');
  composerInput.value = '';
}

async function handleStatusAction(): Promise<void> {
  if (connectionStatus.value.action === 'retry-daemon') {
    await daemonHealth.forceCheck();
    return;
  }
  if (connectionStatus.value.action === 'start-session') {
    emit('new-session');
  }
}

watch(composerInput, () => {
  if (composerError.value) {
    composerError.value = '';
  }
});

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

      <div
        id="home-connection-status"
        class="mb-4 flex max-w-[800px] items-center gap-3 rounded-full border px-3 py-1.5 text-xs"
        :class="connectionStatus.toneClass"
        role="status"
      >
        <span class="size-2 rounded-full" :class="connectionStatus.dotClass" />
        <span class="font-semibold">{{ connectionStatus.label }}</span>
        <span class="text-text-tertiary">{{ connectionStatus.description }}</span>
        <Button
          v-if="connectionStatus.actionLabel"
          type="button"
          variant="ghost"
          size="xs"
          class="ml-1 h-5 rounded-full px-2 text-[11px]"
          @click="handleStatusAction"
        >
          {{ connectionStatus.actionLabel }}
        </Button>
      </div>

      <div class="mb-4 grid w-full max-w-[800px] grid-cols-5 gap-2">
        <Button
          v-for="cat in categories"
          :key="cat.key"
          type="button"
          variant="ghost"
          class="flex h-auto min-h-[72px] flex-col items-center justify-center gap-1.5 whitespace-normal rounded-[8px] border-[1.5px] border-border-card bg-bg-elevated px-1 py-3.5 font-normal transition-all hover:bg-bg-surface-hover"
          @click="handleTileSelect(cat)"
        >
          <span class="text-2xl leading-none" :class="cat.accentClass">{{ cat.icon }}</span>
          <span class="text-center text-[11px] font-semibold text-text-secondary">{{ cat.title }}</span>
          <span class="text-center text-[9px] leading-tight text-text-tertiary">{{ cat.description }}</span>
        </Button>
      </div>

      <Card class="w-full max-w-[800px] rounded-xl py-0 shadow-fluent-8 ring-0 border border-border-card bg-bg-elevated">
        <CardContent class="px-4 py-3">
          <form class="flex items-center gap-2" @submit.prevent="handleComposerSubmit">
            <Pencil class="size-4 shrink-0 text-text-tertiary" :stroke-width="1.5" />
            <Input
              v-model="composerInput"
              type="text"
              placeholder="Ask me anything..."
              :aria-invalid="composerError ? 'true' : 'false'"
              aria-describedby="home-connection-status home-composer-error"
              class="flex-1 border-0 bg-transparent text-sm text-text-primary shadow-none outline-none ring-0 placeholder:text-text-tertiary focus-visible:border-transparent focus-visible:ring-0 h-auto px-0 py-0 rounded-none"
            />
            <Button
              type="submit"
              variant="ghost"
              size="icon"
              :disabled="!canSubmitPrompt"
              class="flex size-9 items-center justify-center rounded-[4px] text-text-tertiary transition-colors hover:text-text-primary"
            >
              <Send class="size-4" :stroke-width="1.5" />
            </Button>
          </form>
          <p
            v-if="composerError"
            id="home-composer-error"
            class="mt-2 text-xs text-danger"
          >
            {{ composerError }}
          </p>
        </CardContent>
      </Card>

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
