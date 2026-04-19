<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { useSessionStore } from '../stores/session';
import type { SavedSession } from '../lib/types';

const emit = defineEmits<{
  resume: [session: SavedSession];
  delete: [sessionId: string];
}>();

const sessionStore = useSessionStore();

const sessions = computed(() => 
  [...sessionStore.savedSessions].sort((a, b) => b.lastUpdated - a.lastUpdated)
);

const liveSessionsByRemoteId = computed(() => {
  const liveSessions = new Map<
    string,
    {
      localId: string;
      isConnected: boolean;
      isConnecting: boolean;
    }
  >();
  for (const [localId, liveSession] of sessionStore.sessions.entries()) {
    const remoteSessionId = liveSession.session?.sessionId;
    if (!remoteSessionId) {
      continue;
    }
    liveSessions.set(remoteSessionId, {
      localId,
      isConnected: liveSession.isConnected,
      isConnecting: liveSession.isConnecting,
    });
  }

  return liveSessions;
});

function formatRelative(dateStr: string | number): string {
  const date = new Date(dateStr);
  const now = Date.now();
  const diffMs = now - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) {
    return 'just now';
  }
  if (diffMin < 60) {
    return `${diffMin}m ago`;
  }
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) {
    return `${diffHr}h ago`;
  }
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay === 1) {
    return 'yesterday';
  }
  if (diffDay < 7) {
    return `${diffDay}d ago`;
  }

  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function handleResume(session: SavedSession) {
  const liveSession = liveSessionsByRemoteId.value.get(session.sessionId);
  if (liveSession) {
    sessionStore.setActiveSession(liveSession.localId);
    return;
  }

  emit('resume', session);
}

function isAgentSession(session: SavedSession): boolean {
  return session.id.startsWith('agent:');
}

function isCurrentConnectedSession(session: SavedSession): boolean {
  return liveSessionsByRemoteId.value.get(session.sessionId)?.isConnected ?? false;
}

function isCurrentConnectingSession(session: SavedSession): boolean {
  return liveSessionsByRemoteId.value.get(session.sessionId)?.isConnecting ?? false;
}

function isResumeUnavailable(session: SavedSession): boolean {
  return !isCurrentConnectedSession(session)
    && !isCurrentConnectingSession(session)
    && session.supportsLoadSession !== true;
}

function getDeleteTitle(session: SavedSession): string {
  if (isCurrentConnectedSession(session)) {
    return 'Disconnect';
  }

  if (isCurrentConnectingSession(session)) {
    return 'Cancel';
  }

  return 'Delete session';
}

async function handleDelete(session: SavedSession, event: Event) {
  event.stopPropagation();
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

  if (confirm('Delete this session?')) {
    emit('delete', session.id);
  }
}
</script>

<template>
  <div>
    <h3 class="mb-3 text-tiny font-normal text-text-tertiary">
      Sessions
    </h3>

    <ul class="m-0 p-0 list-none flex flex-col gap-1.5" role="list" aria-label="Sessions">
      <li v-if="sessions.length === 0" class="p-8 text-muted-foreground text-center" role="listitem">
        <p>No saved sessions yet.</p>
        <p class="mt-2 text-caption">Create a new session to get started.</p>
      </li>
      <li
        v-for="session in sessions"
        :key="session.id"
        role="listitem"
        class="session-item flex cursor-pointer items-center justify-between rounded-[6px] border border-border-card bg-bg-elevated p-2.5 transition-all hover:bg-bg-surface-hover"
        :class="{ 'cursor-default': isResumeUnavailable(session) }"
        @click="handleResume(session)"
      >
        <div class="flex flex-1 flex-col gap-1 min-w-0 overflow-hidden">
          <div class="flex items-center gap-1.5 min-w-0">
            <span
               class="block max-w-[200px] truncate text-sm font-medium text-text-primary"
               :title="session.title"
             >
              {{ session.title }}
            </span>
            <span
              v-if="isCurrentConnectedSession(session)"
               class="connection-state connected inline-flex items-center gap-1 whitespace-nowrap text-tiny"
             >
               <span class="connection-dot" aria-hidden="true" />Connected
             </span>
            <span
              v-else-if="isCurrentConnectingSession(session)"
               class="connection-state connecting inline-flex items-center gap-1 whitespace-nowrap text-tiny"
             >
               <span class="connection-dot" aria-hidden="true" />Connecting
             </span>
            <span
              v-else-if="isResumeUnavailable(session)"
               class="connection-state unavailable inline-flex items-center gap-1 whitespace-nowrap text-tiny"
             >
               <span class="connection-dot" aria-hidden="true" />Resume unavailable
             </span>
          </div>
          <span class="text-tiny text-muted-foreground">
            {{ session.agentName }}
            <span
              v-if="isAgentSession(session)"
              class="inline-block ml-1.5 py-px px-1.5 rounded bg-muted text-muted-foreground text-tiny font-medium lowercase"
            >
              from agent
            </span>
          </span>
          <span class="text-xs text-text-tertiary">{{ formatRelative(session.lastUpdated) }}</span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          class="delete-btn py-1 px-2 border-none rounded-[4px] bg-transparent text-muted-foreground text-title cursor-pointer"
          @click="(e: Event) => handleDelete(session, e)"
          :title="getDeleteTitle(session)"
        >
          ×
        </Button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.session-item {
  transition: background-color 0.15s;
}

.connection-state {
  color: var(--color-text-secondary);
}

.connection-dot {
  width: 6px;
  height: 6px;
  border-radius: 4px;
  background: var(--color-text-muted);
}

.connection-state.connected {
  color: var(--color-success);
}

.connection-state.connected .connection-dot {
  background: var(--color-success);
  animation: pulse-dot 2s ease-in-out infinite;
}

.connection-state.connecting {
  color: var(--color-warning);
}

.connection-state.connecting .connection-dot {
  background: var(--color-warning);
  animation: pulse-dot 1s ease-in-out infinite;
}

.connection-state.unavailable .connection-dot {
  background: var(--color-text-muted);
}

.delete-btn:hover {
  background: color-mix(in srgb, var(--color-danger) 18%, transparent);
  color: var(--color-danger);
}

@keyframes pulse-dot {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.4;
  }
}
</style>
