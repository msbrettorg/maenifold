<script setup lang="ts">
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import {
  Brain,
  CalendarClock,
  GitBranch,
  Home,
  MessageSquare,
  Pin,
  ScrollText,
  Settings,
  ShieldCheck,
  Workflow,
} from 'lucide-vue-next';
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command';
import { TAB_ORDER } from '@/composables/useAppShell';
import { useAppShellStore, type AppShellTab } from '@/stores/app-shell';
import { usePinnedSessionsStore } from '@/stores/pinned-sessions';
import { useSessionBrowserStore } from '@/stores/session-browser';
import { useSessionStore } from '@/stores/session';
import type { AgentSessionSummary } from '@/stores/agent';
import type { SavedSession } from '@/lib/types';

const iconByName = {
  Brain,
  CalendarClock,
  GitBranch,
  Home,
  MessageSquare,
  ScrollText,
  Settings,
  ShieldCheck,
  Workflow,
};

const appShellStore = useAppShellStore();
const sessionBrowserStore = useSessionBrowserStore();
const pinnedSessionsStore = usePinnedSessionsStore();
const sessionStore = useSessionStore();
const { commandPaletteOpen } = storeToRefs(appShellStore);
const { allSessions } = storeToRefs(sessionBrowserStore);
const { pinnedSessionIds } = storeToRefs(pinnedSessionsStore);
const query = ref('');

const recentSessions = computed(() =>
  [...allSessions.value].sort((left, right) => sessionTime(right) - sessionTime(left)),
);

const visibleSessions = computed(() => {
  if (query.value.trim()) {
    return recentSessions.value;
  }

  const pinnedIds = pinnedSessionIds.value;
  const pinned = recentSessions.value.filter((session) => pinnedIds.has(session.sessionId));
  const unpinned = recentSessions.value.filter((session) => !pinnedIds.has(session.sessionId));
  return [...pinned, ...unpinned].slice(0, 10);
});

function sessionTime(session: AgentSessionSummary): number {
  const value = session.lastActivity ?? session.updatedAt ?? session.createdAt;
  return value ? Date.parse(value) || 0 : 0;
}

function sessionTitle(session: AgentSessionSummary): string {
  return session.title?.trim() || `${session.agentName} ${session.sessionId.slice(0, 8)}`;
}

function sessionValue(session: AgentSessionSummary): string {
  return [
    'session',
    sessionTitle(session),
    session.agentName,
    session.cwd,
    session.sessionId,
    session.status,
  ].join(' ');
}

function toSavedSession(session: AgentSessionSummary): SavedSession {
  return {
    id: session.sessionId,
    agentName: session.agentName,
    sessionId: session.sessionId,
    title: sessionTitle(session),
    lastUpdated: sessionTime(session),
    cwd: session.cwd,
  };
}

function closePalette(): void {
  appShellStore.commandPaletteOpen = false;
}

function selectTab(tab: AppShellTab): void {
  closePalette();
  appShellStore.setActiveTab(tab);
}

async function selectSession(session: AgentSessionSummary): Promise<void> {
  closePalette();
  appShellStore.setActiveTab('agents');
  await sessionStore.resumeSession(toSavedSession(session));
}
</script>

<template>
  <CommandDialog :open="commandPaletteOpen" @update:open="appShellStore.commandPaletteOpen = $event">
    <CommandInput v-model="query" placeholder="Search commands or sessions..." />
    <CommandList>
      <CommandEmpty>No results found.</CommandEmpty>
      <CommandGroup heading="Tabs">
        <CommandItem
          v-for="tab in TAB_ORDER"
          :key="tab.value"
          :value="`tab ${tab.label} ${tab.value}`"
          @select="selectTab(tab.value)"
        >
          <component :is="iconByName[tab.icon]" />
          <span>{{ tab.label }}</span>
        </CommandItem>
      </CommandGroup>
      <CommandSeparator />
      <CommandGroup heading="Recent Sessions">
        <CommandItem
          v-for="session in visibleSessions"
          :key="session.sessionId"
          :value="sessionValue(session)"
          @select="selectSession(session)"
        >
          <Pin v-if="!query.trim() && pinnedSessionIds.has(session.sessionId)" />
          <MessageSquare v-else />
          <span class="min-w-0 flex-1 truncate">{{ sessionTitle(session) }}</span>
          <span class="truncate text-xs text-muted-foreground">{{ session.agentName }}</span>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>
</template>
