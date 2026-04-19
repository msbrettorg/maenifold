<script setup lang="ts">
import { computed } from 'vue';
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

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString();
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
    <h3 class="mb-3 text-tiny font-normal text-[var(--text-dim)]">
      Sessions
    </h3>

    <ul class="m-0 p-0 list-none" role="list" aria-label="Sessions">
      <li v-if="sessions.length === 0" class="p-8 text-muted-foreground text-center" role="listitem">
        <p>No saved sessions yet.</p>
        <p class="mt-2 text-caption">Create a new session to get started.</p>
      </li>
      <li
        v-for="session in sessions"
        :key="session.id"
        role="listitem"
        class="session-item mb-1 flex cursor-pointer items-center justify-between rounded-[4px] border-none px-[0.625rem] py-2"
        :class="{ 'cursor-default': isResumeUnavailable(session) }"
        @click="handleResume(session)"
      >
        <div class="flex flex-1 flex-col gap-1 min-w-0 overflow-hidden">
          <div class="flex items-center gap-1.5 min-w-0">
            <span
              class="block max-w-[200px] truncate text-caption font-medium text-foreground"
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
          <span class="text-tiny text-muted-foreground">{{ formatDate(session.lastUpdated) }}</span>
        </div>
        <button
          class="delete-btn py-1 px-2 border-none rounded-[4px] bg-transparent text-muted-foreground text-title cursor-pointer"
          @click="(e) => handleDelete(session, e)"
          :title="getDeleteTitle(session)"
        >
          ×
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.session-item {
  transition: background 0.15s;
}

.session-item:hover {
  background: color-mix(in srgb, var(--muted) 60%, transparent);
}

.connection-state {
  color: var(--muted-foreground);
}

.connection-dot {
  width: 6px;
  height: 6px;
  border-radius: 4px;
  background: var(--text-dim);
}

.connection-state.connected {
  color: var(--sage);
}

.connection-state.connected .connection-dot {
  background: var(--sage);
  animation: pulse-dot 2s ease-in-out infinite;
}

.connection-state.connecting {
  color: var(--amber);
}

.connection-state.connecting .connection-dot {
  background: var(--amber);
  animation: pulse-dot 1s ease-in-out infinite;
}

.connection-state.unavailable .connection-dot {
  background: var(--text-dim);
}

.delete-btn:hover {
  background: color-mix(in srgb, var(--destructive) 18%, transparent);
  color: var(--destructive);
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
