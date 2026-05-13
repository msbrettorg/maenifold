<script setup lang="ts">
import { Button } from '@/components/ui/button';
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

const emit = defineEmits<{
  'select-tab': [tab: AppShellTab];
  'open-tab': [tab: AppShellTab];
  'new-session': [];
}>();

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
</script>

<template>
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
</template>
