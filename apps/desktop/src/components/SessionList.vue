<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { Search } from 'lucide-vue-next';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import SessionRowActions from './SessionRowActions.vue';
import { useSessionStore } from '../stores/session';
import { useAgentStore } from '../stores/agent';
import { usePinnedSessionsStore } from '../stores/pinned-sessions';
import { useSamplingHistoryStore } from '../stores/sampling-history';
import type { SamplingHistoryEntry, SavedSession } from '../lib/types';

const emit = defineEmits<{
  resume: [session: SavedSession];
  delete: [sessionId: string];
}>();
const sessionStore = useSessionStore();
const agentStore = useAgentStore();
const pinnedSessionsStore = usePinnedSessionsStore();
const samplingHistoryStore = useSamplingHistoryStore();
const sessions = computed(() => [...sessionStore.savedSessions]);
const searchQuery = ref('');
const sourceFilters = ['all', 'agent', 'sampling'] as const;
type SourceFilter = (typeof sourceFilters)[number];
const sourceFilter = ref<SourceFilter>('all');

type SessionListRow =
  | { source: 'agent'; id: string; title: string; subtitle: string; lastUpdated: number; session: SavedSession }
  | { source: 'sampling'; id: string; title: string; subtitle: string; lastUpdated: number; session: SamplingHistoryEntry };

const filteredRows = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  const rows: SessionListRow[] = [
    ...sessions.value.map((session) => ({
      source: 'agent' as const,
      id: session.id,
      title: session.title,
      subtitle: session.agentName,
      lastUpdated: session.lastUpdated,
      session,
    })),
    ...samplingHistoryStore.entries.map((session) => ({
      source: 'sampling' as const,
      id: `sampling:${session.samplingRequestId}`,
      title: session.serverName,
      subtitle: session.model,
      lastUpdated: Date.parse(session.createdAt) || 0,
      session,
    })),
  ];
  const matches = rows.filter((row) => {
    if (sourceFilter.value !== 'all' && row.source !== sourceFilter.value) return false;
    if (!q) return true;
    return `${row.title} ${row.subtitle}`.toLowerCase().includes(q);
  });
  return matches.sort((a, b) => {
    const aPinned = a.source === 'agent' && pinnedSessionsStore.isPinned(a.session.id);
    const bPinned = b.source === 'agent' && pinnedSessionsStore.isPinned(b.session.id);
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
  void samplingHistoryStore.start();
});
onBeforeUnmount(() => {
  void agentStore.stop();
  void samplingHistoryStore.stop();
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

function handleRowClick(row: SessionListRow) {
  if (row.source === 'agent') handleResume(row.session);
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

async function handleStopSampling(samplingRequestId: string, event: Event) {
  event.stopPropagation();
  await samplingHistoryStore.cancelSamplingSession(samplingRequestId);
}

function isSamplingStoppable(session: SamplingHistoryEntry): boolean {
  return session.status === 'dispatching' || session.status === 'streaming';
}

function statusLabel(status: string): string {
  return status.split('_').map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ');
}

function showRowActions(row: SessionListRow): boolean {
  return row.source === 'sampling' || !isAgentOnlySession(row.session);
}

function rowDeleteTitle(row: SessionListRow): string | undefined {
  return row.source === 'agent' ? getDeleteTitle(row.session) : undefined;
}

function rowCanStop(row: SessionListRow): boolean {
  return row.source === 'sampling' && isSamplingStoppable(row.session);
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
    <div class="mb-3 flex items-center gap-1" aria-label="Session source filter">
      <Button
        v-for="filter in sourceFilters"
        :key="filter"
        variant="ghost"
        size="sm"
        class="h-7 px-2 text-tiny capitalize"
        :aria-pressed="sourceFilter === filter"
        :class="sourceFilter === filter ? 'bg-bg-surface-hover text-text-primary' : 'text-text-secondary'"
        @click="sourceFilter = filter"
      >
        {{ filter }}
      </Button>
    </div>
    <ScrollArea class="max-h-[calc(100vh-12rem)]">
      <ul class="m-0 p-0 list-none flex flex-col gap-1.5" role="list" aria-label="Sessions">
        <li v-if="sessions.length === 0 && samplingHistoryStore.entries.length === 0" class="p-8 text-muted-foreground text-center" role="listitem">
          <p>No saved sessions yet.</p>
          <p class="mt-2 text-caption">Create a new session to get started.</p>
        </li>
        <li v-else-if="filteredRows.length === 0" class="p-8 text-muted-foreground text-center" role="listitem">
          <p>No sessions match your search.</p>
        </li>
        <li
          v-for="row in filteredRows"
          :key="row.id"
          role="listitem"
          class="flex cursor-pointer items-center justify-between rounded-md border border-border-card bg-bg-elevated p-2.5 transition-colors duration-150 hover:bg-bg-surface-hover"
          :class="{ 'cursor-default': row.source === 'sampling' || (row.source === 'agent' && isResumeUnavailable(row.session)) }"
          @click="handleRowClick(row)"
        >
          <div class="flex flex-1 flex-col gap-1 min-w-0 overflow-hidden">
            <div class="flex items-center gap-1.5 min-w-0">
              <span class="block max-w-[200px] truncate text-sm font-medium text-text-primary" :title="row.title">{{ row.title }}</span>
              <Badge variant="secondary" class="capitalize">{{ row.source }}</Badge>
              <span v-if="row.source === 'agent' && isCurrentConnectedSession(row.session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-success">
                <Skeleton class="size-1.5 rounded-sm bg-success" aria-hidden="true" />Connected
              </span>
              <span v-else-if="row.source === 'agent' && hasActiveAgentSession(row.session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-success">
                <Skeleton class="size-1.5 rounded-sm bg-success" aria-hidden="true" />Active
              </span>
              <span v-else-if="row.source === 'agent' && isCurrentConnectingSession(row.session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-warning">
                <Skeleton class="size-1.5 rounded-sm bg-warning" aria-hidden="true" />Connecting
              </span>
              <span v-else-if="row.source === 'agent' && isResumeUnavailable(row.session)" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-text-secondary">
                <span class="block size-1.5 rounded-sm bg-text-muted" aria-hidden="true" />Resume unavailable
              </span>
              <span v-else-if="row.source === 'sampling'" class="inline-flex items-center gap-1 whitespace-nowrap text-tiny text-text-secondary">
                <Skeleton v-if="row.session.status === 'streaming'" class="size-1.5 rounded-sm bg-success" aria-hidden="true" />
                {{ statusLabel(row.session.status) }}
              </span>
            </div>
            <span class="text-tiny text-muted-foreground inline-flex items-center gap-1.5">
              {{ row.subtitle }}
              <Badge v-if="row.source === 'agent' && isAgentOnlySession(row.session)" variant="secondary" class="lowercase">from agent</Badge>
              <Badge v-if="row.source === 'sampling'" variant="secondary">{{ row.session.runtimeKind }}</Badge>
            </span>
            <span v-if="row.source === 'sampling' && row.session.currentDelta" class="line-clamp-2 text-xs text-text-secondary">
              {{ row.session.currentDelta }}
            </span>
            <span class="text-xs text-text-tertiary">{{ formatRelative(row.lastUpdated) }}</span>
          </div>
          <SessionRowActions
            v-if="showRowActions(row)"
            :session="row.session"
            :source="row.source"
            :delete-title="rowDeleteTitle(row)"
            :can-stop="rowCanStop(row)"
            @delete="handleDelete"
            @stop="handleStopSampling"
          />
        </li>
      </ul>
    </ScrollArea>
  </div>
</template>
