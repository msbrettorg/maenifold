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

  if (session.supportsLoadSession !== true) {
    sessionStore.setError(
      'This ACP service does not advertise session/load, so saved sessions cannot be reopened yet.'
    );
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
  <div class="session-list">
    <h3>Sessions</h3>

    <ul role="list" aria-label="Sessions">
      <li v-if="sessions.length === 0" class="empty-state" role="listitem">
        <p>No saved sessions yet.</p>
        <p class="hint">Create a new session to get started.</p>
      </li>
      <li 
        v-for="session in sessions" 
        :key="session.id"
        role="listitem"
        class="session-item"
        :class="{ unavailable: isResumeUnavailable(session) }"
        @click="handleResume(session)"
      >
        <div class="session-info">
          <div class="session-title-row">
            <span class="session-title" :title="session.title">{{ session.title }}</span>
            <span
              v-if="isCurrentConnectedSession(session)"
              class="connection-badge connected"
            >
              <span class="connection-dot">●</span> connected
            </span>
            <span
              v-else-if="isCurrentConnectingSession(session)"
              class="connection-badge connecting"
            >
              <span class="connection-dot">●</span> connecting
            </span>
            <span
              v-else-if="isResumeUnavailable(session)"
              class="connection-badge unavailable"
            >
              resume unavailable
            </span>
          </div>
          <span class="session-agent">
            {{ session.agentName }}
            <span v-if="isAgentSession(session)" class="session-badge">from agent</span>
          </span>
          <span class="session-date">{{ formatDate(session.lastUpdated) }}</span>
        </div>
        <button 
          class="delete-btn"
          @click="(e) => handleDelete(session, e)"
          :title="getDeleteTitle(session)"
        >
          ×
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>@import '../assets/session-list.css';</style>
