<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { Search } from 'lucide-vue-next';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import SessionRowActions from './SessionRowActions.vue';
import { useSessionStore } from '../stores/session';
import { useAgentStore } from '../stores/agent';
import { usePinnedSessionsStore } from '../stores/pinned-sessions';
import type { SavedSession } from '../lib/types';

const emit = defineEmits<{
  resume: [session: SavedSession];
  delete: [sessionId: string];
}>();
const sessionStore = useSessionStore();
const agentStore = useAgentStore();
const pinnedSessionsStore = usePinnedSessionsStore();
const sessions = computed(() => [...sessionStore.savedSessions]);
const searchQuery = ref('');

const filteredSessions = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  const matches = sessions.value.filter((session) => {
    if (!q) return true;
    return `${session.title} ${session.agentName}`.toLowerCase().includes(q);
  });
  return matches.sort((a, b) => {
    const aPinned = pinnedSessionsStore.isPinned(a.id);
    const bPinned = pinnedSessionsStore.isPinned(b.id);
    if (aPinned !== bPinned) return aPinned ? -1 : 1;
    return b.lastUpdated - a.lastUpdated;
  });
});

const liveSessionsByRemoteId = computed(() => {
  const liveSessions = new Map<string, { localId: string; isConnected: boolean; isConnecting: boolean }>();
  for (const [localId, liveSession] of sessionStore.sessions.entries()) {
    const remoteSessionId = liveSession.session?.sessionId;
    if (!remoteSessionId) continue;
    liveSessions.set(remoteSessionId, {
      localId,
      isConnected: liveSession.isConnected,
      isConnecting: liveSession.isConnecting,
    });
  }
  return liveSessions;
});

onMounted(() => {
  void agentStore.start();
});
onBeforeUnmount(() => {
  void agentStore.stop();
});

function formatRelative(dateStr: string | number): string {
  const date = new Date(dateStr);
  const diffMs = Date.now() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return 'just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay === 1) return 'yesterday';
  if (diffDay < 7) return `${diffDay}d ago`;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function handleResume(session: SavedSession) {
  if (isResumeUnavailable(session)) return;
  const liveSession = liveSessionsByRemoteId.value.get(session.sessionId);
  if (liveSession?.isConnected) {
    sessionStore.setActiveSession(liveSession.localId);
    return;
  }
  emit('resume', session);
}

function isAgentOnlySession(session: SavedSession): boolean { return session.id.startsWith('agent:'); }

function isCurrentConnectedSession(session: SavedSession): boolean {
  return liveSessionsByRemoteId.value.get(session.sessionId)?.isConnected ?? false;
}

function isCurrentConnectingSession(session: SavedSession): boolean {
  return liveSessionsByRemoteId.value.get(session.sessionId)?.isConnecting ?? false;
}

function hasActiveAgentSession(session: SavedSession): boolean {
  return agentStore.activeSessionMap.has(session.sessionId);
}

function isResumeUnavailable(session: SavedSession): boolean {
  return !isCurrentConnectedSession(session)
    && !isCurrentConnectingSession(session)
    && session.supportsLoadSession === false;
}

function getDeleteTitle(session: SavedSession): string {
  if (isCurrentConnectedSession(session)) return `Disconnect '${session.title}'?`;
  if (isCurrentConnectingSession(session)) return `Cancel '${session.title}'?`;
  return `Delete '${session.title}'?`;
}

async function handleDelete(sessionId: string, event: Event) {
  event.stopPropagation();
  const session = sessions.value.find((item) => item.id === sessionId);
  if (!session) return;

  const liveSession = liveSessionsByRemoteId.value.get(session.sessionId);
  if (liveSession?.isConnected) {
    await sessionStore.closeSession(liveSession.localId);
    return;
  }
  if (liveSession?.isConnecting) {
    if (sessionStore.activeSessionId !== liveSession.localId) {
      sessionStore.setActiveSession(liveSession.localId);
    }

    await sessionStore.cancelConnection();
    return;
  }
  emit('delete', session.id);
}

function handleSearchKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') searchQuery.value = '';
}
</script>

<template>
  <div>
    <h3 class="mb-3 text-tiny font-normal text-text-tertiary">
      Sessions
    </h3>
    <div class="relative mb-3">
      <Search class="absolute left-2.5 top-2.5 size-4 text-muted-foreground pointer-events-none" aria-hidden="true" />
      <Input
        v-model="searchQuery"
        class="pl-8"
        placeholder="Filter sessions…"
        aria-label="Filter sessions"
        @keydown="handleSearchKeydown"
      />
    </div>
    <ScrollArea class="max-h-[calc(100vh-12rem)]">
      <ul class="m-0 p-0 list-none flex flex-col gap-1.5" role="list" aria-label="Sessions">
        <li v-if="sessions.length === 0" class="p-8 text-muted-foreground text-center" role="listitem">
          <p>No saved sessions yet.</p>
          <p class="mt-2 text-caption">Create a new session to get started.</p>
        </li>
        <li v-else-if="filteredSessions.length === 0" class="p-8 text-muted-foreground text-center" role="listitem">
          <p>No sessions match your search.</p>
        </li>
        <li
          v-for="session in filteredSessions"
          :key="session.id"
          role="listitem"
          class="flex cursor-pointer items-center justify-between rounded-md border border-border-card bg-bg-elevated p-2.5 transition-colors duration-150 hover:bg-bg-surface-hover"
          :class="{ 'cursor-default': isResumeUnavailable(session) }"
          @click="handleResume(session)"
        >
          <div class="flex flex-1 flex-col gap-1 min-w-0 overflow-hidden">
            <div class="flex items-center gap-1.5 min-w-0">
              <span class="block max-w-[200px] truncate text-sm font-medium text-text-primary" :title="session.title">{{ session.title }}</span>
              <span v-if="isCurrentConnectedSession(session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-success">
                <Skeleton class="size-1.5 rounded-sm bg-success" aria-hidden="true" />Connected
              </span>
              <span v-else-if="hasActiveAgentSession(session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-success">
                <Skeleton class="size-1.5 rounded-sm bg-success" aria-hidden="true" />Active
              </span>
              <span v-else-if="isCurrentConnectingSession(session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-warning">
                <Skeleton class="size-1.5 rounded-sm bg-warning" aria-hidden="true" />Connecting
              </span>
              <span v-else-if="isResumeUnavailable(session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-text-secondary">
                <span class="block size-1.5 rounded-sm bg-text-muted" aria-hidden="true" />Resume unavailable
              </span>
            </div>
            <span class="text-tiny text-muted-foreground inline-flex items-center gap-1.5">
              {{ session.agentName }}
              <Badge v-if="isAgentOnlySession(session)" variant="secondary" class="lowercase">from agent</Badge>
            </span>
            <span class="text-xs text-text-tertiary">{{ formatRelative(session.lastUpdated) }}</span>
          </div>
          <SessionRowActions
            v-if="!isAgentOnlySession(session)"
            :session="session"
            :delete-title="getDeleteTitle(session)"
            @delete="handleDelete"
          />
        </li>
      </ul>
    </ScrollArea>
  </div>
</template>
