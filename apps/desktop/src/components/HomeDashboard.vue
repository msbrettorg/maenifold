<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { Brain, Send, Pencil } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { usePipelineStore } from '../stores/pipeline';
import type { SavedSession } from '../lib/types';
import type { AppShellTab } from '../stores/app-shell';

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
const composerInput = ref('');

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

function handleComposerSubmit() {
  const trimmedPrompt = composerInput.value.trim();
  if (!trimmedPrompt) return;
  emit('prompt-submit', trimmedPrompt);
  emit('select-tab', 'agents');
  emit('open-tab', 'agents');
  composerInput.value = '';
}

onMounted(async () => {
  await pipelineStore.refreshList();
});
</script>

<template>
  <div class="flex flex-1 flex-col items-center px-8 pt-[16vh]">
    <div class="mb-5">
      <Brain class="size-16 text-text-tertiary opacity-40" :stroke-width="1" />
    </div>

    <h1 class="mb-3 text-center text-[26px] font-semibold tracking-tight text-text-primary">What can I help you with?</h1>

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

    <div class="w-full max-w-[800px] rounded-xl border border-border-card bg-bg-elevated px-4 py-3 shadow-fluent-8">
      <form class="flex items-center gap-2" @submit.prevent="handleComposerSubmit">
        <Pencil class="size-4 shrink-0 text-text-tertiary" :stroke-width="1.5" />
        <Input
          v-model="composerInput"
          type="text"
          placeholder="Ask me anything..."
          class="flex-1 border-0 bg-transparent text-sm text-text-primary shadow-none outline-none ring-0 placeholder:text-text-tertiary focus-visible:border-transparent focus-visible:ring-0 h-auto px-0 py-0 rounded-none"
        />
        <Button
          type="submit"
          variant="ghost"
          size="icon"
          class="flex size-9 items-center justify-center rounded-[4px] text-text-tertiary transition-colors hover:text-text-primary"
        >
          <Send class="size-4" :stroke-width="1.5" />
        </Button>
      </form>
    </div>
  </div>
</template>
